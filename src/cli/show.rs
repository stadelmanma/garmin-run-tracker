//! Define show subcommand
use crate::config::Config;
use crate::db::{find_file_by_uuid, open_db_connection};
use rusqlite::{params, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::StructOpt;


/// Show file stats and plot running data
#[derive(Debug, StructOpt)]
pub struct ShowOpts {
    /// Full or partial UUID of file we want to generate route image for (use list-files command
    /// to see UUIDs). The special identifier :last will return the most recent file import.
    #[structopt(name = "FILE_UUID")]
    uuid: String,
    /// name of file to output image data to, if not provided or "-" is used data is written to stdout
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

pub fn show_command(
    config: Config,
    opts: ShowOpts,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
