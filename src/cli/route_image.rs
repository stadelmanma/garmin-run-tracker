//! Define route image subcommand
use crate::config::Config;
use crate::open_db_connection;
use crate::{Error, Location};
use crate::services::visualization::route::Marker;
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
}

pub fn route_image_command(
    config: Config,
    opts: RouteImageOpts,
) -> Result<(), Box<dyn std::error::Error>> {
    let route_drawer = config.get_route_visualization_handler()?;
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

    // TODO throw an eror if the trace is empty

    // fetch all waypoints from record_messages and convert them into a GPS location trace for
    // map plotting
    let mut stmt = conn.prepare(
        "select position_lat, position_long from record_messages where
                                 file_id = ? and
                                 position_lat is not null and
                                 position_long is not null
                                 order by timestamp",
    )?;
    let mut rows = stmt.query(params![file_id])?;
    let mut trace: Vec<Location> = Vec::new();
    while let Some(row) = rows.next()? {
        trace.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
    }

    // fetch all waypoints from lap_messages and convert them into a GPS location markers for
    // map plotting
    let mut stmt = conn.prepare(
        "select end_position_lat, end_position_long from lap_messages where
                                 file_id = ? and
                                 end_position_lat is not null and
                                 end_position_long is not null
                                 order by timestamp",
    )?;
    let mut rows = stmt.query(params![file_id])?;
    let mut markers: Vec<Marker> = vec![Marker::new(trace[0], "S".to_string())];
    let mut mile = 1;
    while let Some(row) = rows.next()? {
        markers.push(Marker::new(
            Location::from_fit_coordinates(row.get(0)?, row.get(1)?),
            format!("{}", mile),
        ));
        mile += 1;
    }
    if let Some(loc) = trace.last() {
        markers.push(Marker::new(*loc, "F".to_string()));
    }

    let image_data = route_drawer.draw_route(&trace, &markers)?;
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
