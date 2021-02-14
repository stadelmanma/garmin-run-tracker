//! Define FIT file update-elevation command
use crate::config::Config;
use crate::db::{find_file_by_uuid, open_db_connection};
use crate::services::{update_elevation_data, ElevationDataSource};
use log::{error, info};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use structopt::StructOpt;

/// Update elevation data for one or more FIT files, all entries with missing elevation data
#[derive(Debug, StructOpt)]
pub struct UpdateElevationOpts {
    /// Full or partial UUIDs of files we want to fetch elevation data for (run with "-l" option
    /// to see UUIDs of files with missing data). The special identifier :last will update the data
    /// for the last file imported
    #[structopt(name = "FILE_UUIDs")]
    uuids: Vec<String>,
    /// List files with missing elevation data, do not update anything
    #[structopt(short = "-l", long)]
    list_missing: bool,
    /// Update all records with missing elevation data
    #[structopt(short = "-a", long)]
    fix_missing: bool,
    /// Overwrite elevation data for the specified files, e.g. if you have a more accurate data source.
    #[structopt(short = "-f", long)]
    overwrite: bool,
}

/// Implementation of the `update-elevation` subcommand
pub fn update_elevation_command(
    config: Config,
    opts: UpdateElevationOpts,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_db_connection()?;

    // return UUIDs of files with missing elevation data but valid lat/long points
    if opts.list_missing {
        return list_missing(&conn);
    }

    // fetch elevation service from config
    let elevation_hdl = match config.get_elevation_handler() {
        Ok(hdl) => hdl,
        Err(e) => {
            error!("Could not initialize the elevation service {}", e);
            return Err(Box::new(e));
        }
    };

    // update elevation data for specified files, we handle each file in it's own transaction
    // so that not everything gets rolled back if it fails. API calls may not be free so we don't
    // want to waste them if possible.
    for uuid in opts.uuids {
        update_file(&mut conn, elevation_hdl.as_ref(), &uuid, opts.overwrite)?;
    }

    // update missing elevation data in database
    if opts.fix_missing {
        info!("Attempting to update elevation data for all database records with missing values");
        let tx = conn.transaction()?;
        update_elevation_data(&tx, elevation_hdl.as_ref(), None, false)?;
        tx.commit()?;
    }

    Ok(())
}

/// Print out the UUIDs of all files with missing elevation data
fn list_missing(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // queries
    let rec_query = "select uuid from files where id in (
        select distinct(file_id)
        from record_messages
        where position_lat is not null and
            position_long is not null and
            elevation is null
    )";
    let lap_query = "select uuid from files where id in (
        select distinct(file_id)
        from lap_messages
        where start_position_lat is not null and
            start_position_long is not null and
            start_elevation is null
    )";
    let mut uuids = HashSet::new();
    let mut stmt = conn.prepare(rec_query)?;
    for uuid in stmt.query_map(params![], |row| row.get::<usize, String>(0))? {
        uuids.insert(uuid?);
    }
    let mut stmt = conn.prepare(lap_query)?;
    for uuid in stmt.query_map(params![], |row| row.get::<usize, String>(0))? {
        uuids.insert(uuid?);
    }

    if uuids.is_empty() {
        println!("No files have missing elevation data.");
    } else {
        println!(
            "The following {:?} files have missing elevation data:",
            uuids.len()
        );
        for uuid in uuids {
            println!(" *\t{}", uuid);
        }
    }

    Ok(())
}

/// Update the elevation data for a file, this suppresses all non-fatal errors and instead
/// emits out logging messages for them.
fn update_file<T: ElevationDataSource + ?Sized>(
    conn: &mut Connection,
    elevation_hdl: &T,
    uuid: &str,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // locate file_id from uuid
    let file_info = match find_file_by_uuid(&conn, &uuid) {
        Ok(info) => info,
        Err(e) => return Err(Box::new(e)),
    };
    if file_info.id().is_none() {
        error!(
            "File with UUID={} has no file_id cannot update elevation data.",
            file_info.uuid()
        );
        return Ok(());
    }

    let tx = conn.transaction()?;
    match update_elevation_data(&tx, elevation_hdl, file_info.id(), overwrite) {
        Ok(_) => {
            tx.commit()?;
            info!(
                "Successfully updated elevation for FIT file '{}'",
                file_info.uuid()
            );
        }
        Err(e) => {
            tx.rollback()?;
            error!(
                "Could not import elevation data from the API for FIT file '{}'",
                file_info.uuid()
            );
            error!("{}", e);
        }
    }

    Ok(())
}
