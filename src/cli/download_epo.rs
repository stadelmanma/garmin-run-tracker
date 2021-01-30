//! Define the download-epo subcommand
//! Original source of code: https://github.com/scrapper/postrunner/blob/master/lib/postrunner/EPO_Downloader.rb
use crate::config::Config;
use crate::Error;
use chrono::{Duration, Local, TimeZone, Utc};
use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::StructOpt;

static URI: &str = "https://omt.garmin.com/Rce/ProtobufApi/EphemerisService/GetEphemerisData";
// This is the payload of the POST request. It was taken from
// http://www.kluenter.de/garmin-ephemeris-files-and-linux/. It may contain
// a product ID or serial number.
static POST_DATA: &[u8] = &[
    10, 45, 10, 7, 101, 120, 112, 114, 101, 115, 115, 18, 5, 100, 101, 95, 68, 69, 26, 7, 87, 105,
    110, 100, 111, 119, 115, 34, 18, 54, 48, 49, 32, 83, 101, 114, 118, 105, 99, 101, 32, 80, 97,
    99, 107, 32, 49, 18, 10, 8, 140, 180, 147, 184, 14, 18, 0, 24, 0, 24, 28, 34, 0,
];

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
    let epo_data = strip_leading_bytes(epo_data)?;
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
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&format!("{}", POST_DATA.len()))?,
    );

    let client = Client::new();
    let resp = client.post(URI).headers(headers).body(POST_DATA).send()?;
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

/// The downloaded data contains Extended Prediction Orbit data for 6 hour
/// windows for 7 days. Each EPO set is 2307 bytes long, but the first 3
/// bytes must be removed for the FR620 to understand it.
/// https://forums.garmin.com/apps-software/mac-windows-software/f/garmin-express/71291/when-will-garmin-express-mac-be-able-to-sync-gps-epo-bin-file-on-fenix-2
/// The 2304 bytes consist of 32 sets of 72 byte GPS satellite data.
/// http://www.vis-plus.ee/pdf/SIM28_SIM68R_SIM68V_EPO-II_Protocol_V1.00.pdf
fn strip_leading_bytes(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    if data.len() != 28 * 2307 {
        let msg = format!(
            "EPO data has unexpected length of {} bytes instead of {}",
            data.len(),
            28 * 2307
        );
        error!("{}", &msg);
        return Err(Error::Other(msg));
    }

    // remove the 3 leading bytes of each 2307 byte chunk
    let mut fixed = Vec::with_capacity(28 * 2304);
    for chk in data.chunks(2307) {
        // the post runner code checked that the fill bytes were all 0 but the logic is broken due
        // to a faulty type conversion. This doesn't appear to be the case anymore but in the file
        // I downloaded they were all 10, 128, 192.
        fixed.extend_from_slice(&chk[3..]);
    }

    if fixed.len() != 28 * 2304 {
        let msg = format!(
            "Fixed EPO data has unexpected length of {} bytes instead of {}",
            fixed.len(),
            28 * 2304
        );
        error!("{}", &msg);
        return Err(Error::Other(msg));
    }

    Ok(fixed)
}

/// Verify the checksum and the timestamps in the EPO data
fn validate_epo_data(data: &[u8]) -> Result<(), Error> {
    // timestamps in EPO data use this as the reference point
    let ref_date = Utc.ymd(1980, 1, 6).and_hms(0, 0, 0);
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
        if date > now + Duration::hours(8 * 24) {
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
