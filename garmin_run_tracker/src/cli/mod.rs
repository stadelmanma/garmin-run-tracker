//! Define the application's command line interface
use crate::config::Config;
use chrono::NaiveDate;
use simplelog::LevelFilter;
use structopt::StructOpt;

mod download_epo;
use download_epo::{download_epo_command, DownloadEpoOpts};
mod import;
use import::{import_command, ImportOpts};
mod list_files;
use list_files::{list_files_command, ListFilesOpts};
mod route_image;
use route_image::{route_image_command, RouteImageOpts};
mod show;
use show::{show_command, ShowOpts};
mod update_elevation;
use update_elevation::{update_elevation_command, UpdateElevationOpts};

/// Parse FIT formatted files and import their data into the local database
#[derive(Debug, StructOpt)]
pub struct Cli {
    /// Set logging level to debug, use a second time (e.g. -vv) to set logging to trace
    #[structopt(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Suppress info logging messages use a second time (e.g. -qq) to hide warnings
    #[structopt(short, long, parse(from_occurrences))]
    quiet: i32,
    /// Additional commands beyond importing data
    #[structopt(subcommand)]
    cmd: Command,
}

impl Cli {
    /// Return the verbose flag counts as a log level filter
    pub fn verbosity(&self, default: LevelFilter) -> LevelFilter {
        if self.quiet == 1 {
            LevelFilter::Warn
        } else if self.quiet > 1 {
            LevelFilter::Error
        } else if self.verbose == 1 {
            LevelFilter::Debug
        } else if self.verbose > 1 {
            LevelFilter::Trace
        } else {
            default
        }
    }

    /// Consume options struct and return the result of subcommand execution
    pub fn execute_subcommand(self, config: Config) -> Result<(), Box<dyn std::error::Error>> {
        self.cmd.execute(config)
    }
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Update the Extended Prediction Orbit (EPO) data for one or more garmin devices
    #[structopt(name = "download-epo")]
    DownloadEpo(DownloadEpoOpts),
    /// Import new FIT files into the application
    #[structopt(name = "import")]
    Import(ImportOpts),
    /// List files stored in the database
    #[structopt(name = "list-files")]
    Listfiles(ListFilesOpts),
    /// Create a route image from the GPS trace
    #[structopt(name = "route-image")]
    RouteImage(RouteImageOpts),
    /// Show file statistics and plot running data
    #[structopt(name = "show")]
    Show(ShowOpts),
    /// Update elevation data in the database for one or more files
    #[structopt(name = "update-elevation")]
    UpdateElevation(UpdateElevationOpts),
}

impl Command {
    /// Consume enum variant and return the result of the command's execution
    fn execute(self, config: Config) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Command::DownloadEpo(opts) => download_epo_command(config, opts),
            Command::Import(opts) => import_command(config, opts),
            Command::Listfiles(opts) => list_files_command(opts),
            Command::RouteImage(opts) => route_image_command(config, opts),
            Command::Show(opts) => show_command(config, opts),
            Command::UpdateElevation(opts) => update_elevation_command(config, opts),
        }
    }
}

fn parse_date(src: &str) -> Result<NaiveDate, chrono::format::ParseError> {
    NaiveDate::parse_from_str(src, "%Y-%m-%d")
}
