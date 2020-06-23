use garmin_run_tracker::elevation::OpenTopoData;
use garmin_run_tracker::{create_database, import_fit_data, update_elevation_data, Error};
use log::{error, info, trace, warn};
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

/// Parse FIT formatted files and import their data into the local database
#[derive(Debug, StructOpt)]
#[structopt(name = "fit_to_json")]
struct Cli {
    /// FIT files to import
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
    /// A level of verbosity, and can be used up to three times for maximum logging (e.g. -vvv)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Silently ignore duplicate files and emit no messages
    #[structopt(long)]
    ignore_duplicate_files: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::from_args();
    let level_filter = if opt.verbose == 1 {
        LevelFilter::Info
    } else if opt.verbose == 2 {
        LevelFilter::Debug
    } else if opt.verbose > 2 {
        LevelFilter::Trace
    } else {
        LevelFilter::Warn
    };
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
    let num_files = opt.files.len();
    for file in opt.files {
        trace!("Importing FIT file: {:?}", file);
        let mut fp = File::open(&file)?;
        let uuid = match import_fit_data(&mut fp) {
            Ok(uuid) => uuid,
            Err(e) => {
                // handle various errors
                match &e {
                    Error::DuplicateFileError(_) => {
                        if opt.ignore_duplicate_files {
                            trace!("{}", e);
                            continue;
                        } else if num_files == 1 {
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

    Ok(())
}
