//! Define the application's command line interface
use chrono::NaiveDate;
use simplelog::LevelFilter;
use std::path::PathBuf;
use structopt::StructOpt;

mod list_files;
use list_files::{list_files_command, ListFilesOpts};
mod route_image;
use route_image::{route_image_command, RouteImageOpts};

/// Parse FIT formatted files and import their data into the local database
#[derive(Debug, StructOpt)]
pub struct Cli {
    /// FIT files to import
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
    /// A level of verbosity, and can be used up to three times for maximum logging (e.g. -vvv)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: i32,
    /// Silently ignore duplicate files and emit no messages
    #[structopt(long)]
    ignore_duplicate_files: bool,
    /// Attempt to pull elevation data for rows in the database that are currently NULL
    #[structopt(long)]
    fix_missing_elevation: bool,
    /// Additional commands beyond importing data
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

impl Cli {
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Return the verbose flag counts as a log level filter
    pub fn verbosity(&self) -> LevelFilter {
        if self.verbose == 1 {
            LevelFilter::Info
        } else if self.verbose == 2 {
            LevelFilter::Debug
        } else if self.verbose > 2 {
            LevelFilter::Trace
        } else {
            LevelFilter::Warn
        }
    }

    pub fn ignore_duplicate_files(&self) -> bool {
        self.ignore_duplicate_files
    }

    pub fn fix_missing_elevation(&self) -> bool {
        self.fix_missing_elevation
    }

    /// Consume options struct and return the result of subcommand execution
    pub fn execute_subcommand(self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(cmd) = self.cmd {
            cmd.execute()
        } else {
            // No subcommand to execute
            Ok(())
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// List files stored in the database
    #[structopt(name = "list-files")]
    Listfiles(ListFilesOpts),
    #[structopt(name = "route-image")]
    RouteImage(RouteImageOpts),
}

impl Command {
    /// Consume enum variant and return the result of the command's execution
    fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Command::Listfiles(opts) => list_files_command(opts),
            Command::RouteImage(opts) => route_image_command(opts),
        }
    }
}

fn parse_date(src: &str) -> Result<NaiveDate, chrono::format::ParseError> {
    NaiveDate::parse_from_str(src, "%Y-%m-%d")
}
