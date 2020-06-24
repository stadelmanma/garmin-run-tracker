use garmin_run_tracker::cli::Cli;
use garmin_run_tracker::elevation::OpenTopoData;
use garmin_run_tracker::{create_database, import_fit_data, update_elevation_data, Error};
use log::{error, info, trace, warn};
use simplelog::{Config, TermLogger, TerminalMode};
use std::fs::File;
use structopt::StructOpt;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::from_args();
    let level_filter = opt.verbosity();
    TermLogger::init(level_filter, Config::default(), TerminalMode::Mixed)?;

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
        let uuid = match import_fit_data(&mut fp) {
            Ok(uuid) => uuid,
            Err(e) => {
                // handle various errors
                match &e {
                    Error::DuplicateFileError(_) => {
                        if opt.ignore_duplicate_files() {
                            trace!("{}", e);
                            continue;
                        } else if opt.files().len() == 1 {
                            // if we are importing a single file and it's a dupe throw a hard error
                            error!("{}", e);
                            return Err(Box::new(e));
                        } else {
                            // if we are impoting multiple files, just warn about the dupe
                            warn!("{}", e);
                            continue;
                        }
                    }
                    _ => return Err(Box::new(e)),
                }
            }
        };

        info!(
            "Successfully imported FIT file: {:?} (UUID={})",
            &file, &uuid
        );
        if let Err(e) = update_elevation_data(&topo, &uuid) {
            error!(
                "Could not import elevation data from the API for FIT file '{}'",
                &uuid
            );
            error!("{}", e)
        } else {
            info!("Successfully imported elevation for FIT file '{}'", &uuid);
        }
    }

    // execute any subcommands
    opt.execute_subcommand()
}
