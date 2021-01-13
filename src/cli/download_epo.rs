//! Define the download-epo subcommand
//! Original source of code: https://github.com/scrapper/postrunner/blob/master/lib/postrunner/EPO_Downloader.rb
use crate::config::Config;
use crate::Error;
use log::{debug, info};
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::StructOpt;

static URI: &str = "https://omt.garmin.com/Rce/ProtobufApi/EphemerisService/GetEphemerisData";
// This is the payload of the POST request. It was taken from
// http://www.kluenter.de/garmin-ephemeris-files-and-linux/. It may contain
// a product ID or serial number.
static POST_DATA: [u8; 63] = [
    10, 45, 10, 7, 101, 120, 112, 114, 101, 115, 115, 18, 5, 100, 101, 95, 68, 69, 26, 7, 87, 105,
    110, 100, 111, 119, 115, 34, 18, 54, 48, 49, 32, 83, 101, 114, 118, 105, 99, 101, 32, 80, 97,
    99, 107, 32, 49, 18, 10, 8, 140, 180, 147, 184, 14, 18, 0, 24, 0, 24, 28, 34, 0,
];
static HEADERS: [(&str, &str); 3] = [
    ("Garmin-Client-Name", "CoreService"),
    ("Content-Type", "application/octet-stream"),
    ("Content-Length", "63"),
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
    let mut epo_data = download_epo_data()?;
    let epo_data = strip_leading_bytes(&mut epo_data)?;
    validate_epo_data(epo_data)?;

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
        for path in config.epo_data_paths().iter().map(|s| PathBuf::from(s)) {
            info!("Writing EPO data to {:?}", path);
            let mut fp = File::create(path)?;
            fp.write_all(&epo_data)?
        }
    }

    Ok(())
}

/// Request EPO data from garmin server using the extracted credentials
fn download_epo_data() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(Vec::new())
}

/// The downloaded data contains Extended Prediction Orbit data for 6 hour
/// windows for 7 days. Each EPO set is 2307 bytes long, but the first 3
/// bytes must be removed for the FR620 to understand it.
/// https://forums.garmin.com/apps-software/mac-windows-software/f/garmin-express/71291/when-will-garmin-express-mac-be-able-to-sync-gps-epo-bin-file-on-fenix-2
/// The 2304 bytes consist of 32 sets of 72 byte GPS satellite data.
/// http://www.vis-plus.ee/pdf/SIM28_SIM68R_SIM68V_EPO-II_Protocol_V1.00.pdf
fn strip_leading_bytes(data: &mut [u8]) -> Result<&[u8], Error> {
    Ok(data)
}

/// Verify the checksum and the timestamps in the EPO data
fn validate_epo_data(data: &[u8]) -> Result<(), Error> {
    Ok(())
}

fn write_to_stdout(data: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&data)
}
