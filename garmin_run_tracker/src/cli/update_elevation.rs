//! Define FIT file update-elevation command
use crate::config::Config;
use crate::db::{find_file_by_uuid, open_db_connection};
use crate::services::update_elevation_data;
use log::{error, info};
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
    // fetch elevation service from config
    let elevation_hdl = match config.get_elevation_handler() {
        Ok(hdl) => hdl,
        Err(e) => {
            error!("Could not initialize the elevation service {}", e);
            return Err(Box::new(e));
        }
    };
    let mut conn = open_db_connection()?;

    // return UUIDs of files with missing elevation data but valid lat/long points
    if opts.list_missing {
        unimplemented!();
        return Ok(());
    }

    // update elevation data for specified files, we handle each file in it's own transaction
    // so that not everything gets rolled back if it fails. API calls may not be free so we don't
    // want to waste them if possible.
    for uuid in opts.uuids {
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
            continue;
        }

        let tx = conn.transaction()?;
        match update_elevation_data(&tx, elevation_hdl.as_ref(), file_info.id(), opts.overwrite) {
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
