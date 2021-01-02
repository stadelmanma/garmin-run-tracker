//! Define route image subcommand
use crate::open_db_connection;
use crate::visualization::route::{OpenMapTiles, RouteDrawingService};
use crate::{Error, Location};
use rusqlite::{params, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::StructOpt;

/// Generate an image of the running route based on the file's waypoints
#[derive(Debug, StructOpt)]
pub struct RouteImageOpts {
    /// UUID of file we want to generate route info for (use list-files command to see UUIDs)
    #[structopt(name = "FILE_UUID")]
    uuid: String,
    /// name of file to output image data to, if "-" is used we will write to stdout
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
    /// struct used to generate the route image
    #[structopt(skip)]
    route_drawer: OpenMapTiles,
}

pub fn route_image_command(opts: RouteImageOpts) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db_connection()?;

    // locate file_id from uuid
    let file_id = match conn.query_row(
        "select id from files where uuid = ?",
        params![opts.uuid],
        |r| r.get::<usize, i32>(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return Err(Box::new(Error::FileDoesNotExistError(
                opts.uuid.to_string(),
            )));
        }
    };

    // fetch all waypoints from record_messages
    // convert all waypoints to a Location vector (see OpenTopoData)
    let mut stmt = conn.prepare(
        "select position_lat, position_long from record_messages where
                                 file_id = ? and
                                 position_lat is not null and
                                 position_long is not null",
    )?;
    let mut rows = stmt.query(params![file_id])?;
    let mut trace: Vec<Location> = Vec::new();
    while let Some(row) = rows.next()? {
        trace.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
    }

    let image_data = opts.route_drawer.draw_route(&trace)?;
    if let Some(path) = opts.output {
        if path.to_string_lossy() == "-" {
            write_to_stdout(&image_data)?
        } else {
            let mut fp = File::create(path)?;
            fp.write_all(&image_data)?
        }
    } else {
        write_to_stdout(&image_data)?
    }

    Ok(())
}

fn write_to_stdout(data: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&data)
}