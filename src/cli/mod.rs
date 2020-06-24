//! Define the application's command line interface
use chrono::NaiveDate;
use simplelog::LevelFilter;
use structopt::StructOpt;
use std::path::PathBuf;

mod list_files;
pub use list_files::{ListFilesOpts, list_files_command};

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
    /// Additional commands beyond importing data
    #[structopt(subcommand)]
    cmd: Option<Command>
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

    /// Consume options struct and return the result of subcommand execution
    pub fn execute_subcommand(self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(cmd) = self.cmd {
            match cmd {
                Command::Listfiles(opts) => list_files_command(opts)
            }
        }
        else {
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
}

fn parse_date(src: &str) -> Result<NaiveDate, chrono::format::ParseError> {
    NaiveDate::parse_from_str(src , "%Y-%m-%d")
}
