use garmin_run_tracker::cli::Cli;
use garmin_run_tracker::services::elevation::OpenTopoData;
use garmin_run_tracker::services::update_elevation_data;
use garmin_run_tracker::{create_database, devices_dir, import_fit_data, Error};
use log::{error, info, trace, warn};
use simplelog::{Config, TermLogger, TerminalMode};
use std::fs::{copy as copy_file, create_dir_all, File};
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::from_args();
    let level_filter = opt.verbosity();
    TermLogger::init(level_filter, Config::default(), TerminalMode::Mixed)?;

    // create data_dir if needed
    if !devices_dir().exists() {
        create_dir_all(devices_dir())?;
    }

    // create database if needed
    create_database()?;

    // use a locally hosted instance of opentopodata as the elevation data source
    // we have it configured to use the ned10m dataset and a max request batch size of
    // 100.
    let topo = OpenTopoData::new(
        "http://localhost:5000".to_string(),
        "ned10m".to_string(),
        100,
    );

    // Import each fit file
    for file in opt.files() {
        trace!("Importing FIT file: {:?}", file);
        let mut fp = File::open(&file)?;
        let file_info = match import_fit_data(&mut fp) {
            Ok(file_info) => file_info,
            Err(e) => {
                // handle various errors
                match &e {
                    Error::DuplicateFileError(_) => {
                        if opt.files().len() == 1 {
                            // if we are importing a single file and it's a dupe throw a hard error
                            error!("{}", e);
                            return Err(Box::new(e));
                        } else {
                            // if we are importing multiple files, just warn about the dupe
                            warn!("{}", e);
                            continue;
                        }
                    }
                    _ => return Err(Box::new(e)),
                }
            }
        };

        // copy FIT file to a local storage location since the device itself will delete the
        // file when it needs space.
        if !opt.no_copy() {
            let sub_dir_name = format!(
                "{}-{}-{}",
                file_info.manufacturer(),
                file_info.product(),
                file_info.serial_number()
            );
            let mut dest = devices_dir().join(&sub_dir_name);
            if !dest.exists() {
                create_dir_all(&dest)?;
            }
            match file.file_name() {
                Some(name) => dest.push(name),
                None => dest.push(&format!("{}.fit", file_info.uuid())),
            };
            copy_file(file, &dest)?;
            info!("Successfully copied FIT file {:?} to {:?}", &file, &dest);
        }

        info!(
            "Successfully imported FIT file: {:?} (UUID={})",
            &file,
            file_info.uuid()
        );
        // add elevation data if possible, we overwrite here on the assumption that API is
        // more accurate value than the device.
        info!(
            "Attempting to update elevation data for FIT file: {:?} (UUID={})...",
            &file,
            file_info.uuid()
        );
        if let Err(e) = update_elevation_data(&topo, Some(file_info.uuid()), true) {
            error!(
                "Could not import elevation data from the API for FIT file '{}'",
                file_info.uuid()
            );
            error!("{}", e)
        } else {
            info!(
                "Successfully imported elevation for FIT file '{}'",
                file_info.uuid()
            );
        }
    }

    // update missing elevation data in database, we'll hard error here if this fails since
    // the task was requested directly, overwrite = false to only hit missed values
    if opt.fix_missing_elevation() {
        update_elevation_data(&topo, None, false)?;
    }

    // execute any subcommands
    opt.execute_subcommand()
}
