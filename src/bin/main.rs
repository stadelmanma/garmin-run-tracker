use garmin_run_tracker::{create_database, import_fit_data};
use log::{trace, info};
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
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Cli::from_args();
    let level_filter = if opt.verbose == 1 {
        LevelFilter::Info
    }
    else if opt.verbose == 2 {
        LevelFilter::Debug
    }
    else if opt.verbose > 2 {
        LevelFilter::Trace
    }
    else {
        LevelFilter::Warn
    };
    TermLogger::init(level_filter, Config::default(), TerminalMode::Mixed)?;

    // create database if needed
    create_database()?;

    // Read each FIT file and import it
    for file in opt.files {
        // open file and parse data
        let mut fp = File::open(&file)?;
        let data = fitparser::from_reader(&mut fp)?;
        trace!("Parsed FIT file: {:?} and found {} messages", file, data.len());
        import_fit_data(&data)?;
        info!("Successfully imported FIT file: {:?}", file);
    }

    Ok(())
}
