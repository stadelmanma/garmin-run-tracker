//! Define the download-epo subcommand
//! Original source of code: https://github.com/StevenMaude/armstrong/blob/main/armstrong.go
use crate::config::Config;
use crate::Error;
use chrono::{Duration, Local, TimeZone, Utc};
use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::StructOpt;

// Garmin forum post:
// "…each EPO SET is 2304 bytes"
static EPO_LENG: usize = 2304;

static URI: &str = "https://epodownload.mediatek.com/EPO.DAT";

/// Download Extended Prediction Orbit (EPO) data for one or more garmin devices
#[derive(Debug, StructOpt)]
pub struct DownloadEpoOpts {
    /// Name of file to output EPO data to, when this option is used the config defined `epo_data_paths`
    /// will be ignored. If "-" is used we will write to stdout.
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

/// Download Extended Prediction Orbit (EPO) data for one or more garmin devices
pub fn download_epo_command(
    config: Config,
    opts: DownloadEpoOpts,
) -> Result<(), Box<dyn std::error::Error>> {
    // download, fix and validate the EPO data
    let epo_data = download_epo_data()?;
    if epo_data.len() != 120 * EPO_LENG {
        let msg = format!("EPO data has unexpected length: {:?}", epo_data.len());
        error!("{}", &msg);
        return Err(Box::new(Error::Other(msg)));
    }

    let epo_data = trim_epo_data(epo_data);
    validate_epo_data(&epo_data)?;

    // output the EPO data to a single file or the config defined locations
    if let Some(path) = opts.output {
        if path.to_string_lossy() == "-" {
            debug!("Writing EPO data to STDOUT");
            write_to_stdout(&epo_data)?
        } else {
            debug!("Writing EPO data to {:?}", path);
            let mut fp = File::create(path)?;
            fp.write_all(&epo_data)?
        }
    } else {
        for path in config.epo_data_paths().iter().map(PathBuf::from) {
            info!("Writing EPO data to {:?}", path);
            match File::create(&path) {
                Ok(mut fp) => fp.write_all(&epo_data)?,
                Err(e) => {
                    // emit warning message but swallow actual failure in case we have multiple
                    // paths to write to and not all devices are mounted
                    warn!("Could not write data to {:?} - {}", path, e);
                }
            }
        }
    }

    Ok(())
}

/// Request EPO data from garmin server using the extracted credentials
fn download_epo_data() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // setup headers
    let mut headers = HeaderMap::with_capacity(3);
    headers.insert(
        HeaderName::from_static("garmin-client-name"),
        HeaderValue::from_static("CoreService"),
    );

    let client = Client::new();
    let resp = client.get(URI).headers(headers).send()?;
    if resp.status().is_success() {
        // return EPO data
        match resp.bytes() {
            Ok(data) => Ok(data.into_iter().collect()),
            Err(e) => Err(Box::new(e)),
        }
    } else {
        let code = resp.status();
        Err(Box::new(Error::RequestError(
            code,
            format!("Failed to download EPO data from {}", URI),
        )))
    }
}

/// trims the MediaTek data sufficiently for a Garmin watch.
fn trim_epo_data(data: Vec<u8>) -> Vec<u8> {
    // Garmin forum post:
    // "…even with such a clean file, the Garmin watches use only a max of one-digit
    // days, ie 9 days of data…"
    // "…each EPO SET … has 6 hours of satellite locations…" so 4 per day
    let nbytes = 9 * 4 * EPO_LENG;
    data[..nbytes].to_vec()
}

/// Verify the checksum and the timestamps in the EPO data
fn validate_epo_data(data: &[u8]) -> Result<(), Error> {
    // timestamps in EPO data use this as the reference point
    let ref_date = Utc.with_ymd_and_hms(1980, 1, 6, 0, 0, 0).unwrap();
    let now = Utc::now();
    let mut start_date = now;
    let mut end_date = ref_date;

    // spit data into 72 byte chunks, each chunk represents data for a single satilite
    let mut offset = 0;
    for sat in data.chunks(72) {
        // the last byte is a checksum of the first 71
        let xor = sat[..71].iter().fold(0, |xor, v| xor ^ v);
        if xor != sat[71] {
            let msg = format!("Checksum error in EPO data at offset {}", offset);
            error!("{}", &msg);
            return Err(Error::Other(msg));
        }

        // The first 3 bytes of every satellite record look like a timestamp.
        // I assume they are hours after January 6th, 1980 UTC. They probably
        // indicate the start of the 6 hour window that the data is for.
        let hours_after = sat[0] as i64 | ((sat[1] as i64) << 8) | ((sat[2] as i64) << 16);
        let date = ref_date + Duration::hours(hours_after);
        if date > now + Duration::hours(9 * 24) {
            warn!("EPO timestamp ({:?}) is too far in the future", date);
        } else if date < now - Duration::hours(24) {
            warn!("EPO timestamp ({:?}) is too old", date);
        }
        if date < start_date {
            start_date = date;
        }
        if date > end_date {
            end_date = date;
        }
        offset += 72;
    }
    info!(
        "EPO data is valid from {} - {}",
        Local::from_utc_datetime(&Local, &start_date.naive_utc()).format("%m/%d"),
        Local::from_utc_datetime(&Local, &(end_date + Duration::hours(6)).naive_utc())
            .format("%m/%d")
    );

    Ok(())
}

fn write_to_stdout(data: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&data)
}
