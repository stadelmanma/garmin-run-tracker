use garmin_run_tracker::cli::Cli;
use garmin_run_tracker::{create_database, devices_dir, load_config};
use simplelog::{Config as LoggerConfig, TermLogger, TerminalMode};
use std::fs::create_dir_all;
use structopt::StructOpt;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // create data_dir if needed
    if !devices_dir().exists() {
        create_dir_all(devices_dir())?;
    }

    // create database if needed
    create_database()?;

    // load config now so that the other initialization tasks can complete. They aren't currently
    // dependent on the config file but if that changes we will need to reorder stuff.
    let config = load_config()?;

    let opt = Cli::from_args();
    let log_level = opt.verbosity(config.log_level());
    TermLogger::init(log_level, LoggerConfig::default(), TerminalMode::Mixed)?;

    // execute any subcommands
    opt.execute_subcommand(config)
}

/// wrap actual "main" function so we can format errors with Display instead of Debug
fn main() {
    std::process::exit(match run() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("Error: {}", err);
            1
        }
    });
}
