//! Define the list-files subcommand
use super::parse_date;
use crate::open_db_connection;
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Result, NO_PARAMS};
use structopt::StructOpt;

/// List all files in the local database
#[derive(Debug, StructOpt)]
pub struct ListFilesOpts {
    /// Date to list files for, or the start date if an end date is used (YYYY-MM-DD format)
    #[structopt(name = "DATE", parse(try_from_str = parse_date))]
    start_date: Option<NaiveDate>,
    /// End of date range to list files for (YYYY-MM-DD format)
    #[structopt(name = "END_DATE", parse(try_from_str = parse_date))]
    end_date: Option<NaiveDate>,
    // TODO: output summary about files listed
}

pub fn list_files_command(opts: ListFilesOpts) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db_connection()?;
    let mut stmt;
    let mut rows = if let Some(start_date) = opts.start_date {
        stmt = conn.prepare(
            "select time_created, device_manufacturer, device_product, uuid from files
                where time_created between ? and ?
                order by time_created",
        )?;
        if let Some(end_date) = opts.end_date {
            stmt.query(params![start_date, end_date.and_hms(23, 59, 59)])?
        } else {
            // if no end date is provided return all for the given day
            stmt.query(params![start_date, start_date.and_hms(23, 59, 59)])?
        }
    } else {
        stmt = conn.prepare(
            "select time_created, device_manufacturer, device_product, uuid from files
                order by time_created",
        )?;
        stmt.query(NO_PARAMS)?
    };

    println!("Date, Device, UUID");
    while let Some(row) = rows.next()? {
        // make this a nicely formatted local time
        let timestamp: DateTime<Local> = row.get(0)?;
        // for some reason these are getting spit out as Blob instead of text
        let manufacturer = row
            .get::<usize, Vec<u8>>(1)
            .map(|v| String::from_utf8(v))??;
        let product: String = row
            .get::<usize, Vec<u8>>(2)
            .map(|v| String::from_utf8(v))??;
        let uuid: String = row.get(3)?;

        println!(
            "{} {}-{} ({})",
            timestamp.format("%Y-%m-%d %H:%M"),
            manufacturer,
            product,
            uuid
        );
    }

    Ok(())
}
