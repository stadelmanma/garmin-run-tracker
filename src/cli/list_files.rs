//! Define the list-files subcommand
use super::parse_date;
use crate::db::{open_db_connection, QueryStringBuilder};
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection, Result, NO_PARAMS};
use std::collections::HashMap;
use std::convert::TryFrom;
use structopt::StructOpt;

/// List all files in the local database
#[derive(Debug, StructOpt)]
pub struct ListFilesOpts {
    /// Output per file statistics
    #[structopt(short, long)]
    stat: bool,
    /// List files after the specified date (YYYY-MM-DD format)
    #[structopt(short="-S", long, parse(try_from_str = parse_date))]
    since: Option<NaiveDate>,
    /// List files before the specified date (YYYY-MM-DD format)
    #[structopt(short="-U", long, parse(try_from_str = parse_date))]
    until: Option<NaiveDate>,
    /// Reverse file ordering to be new -> old
    #[structopt(short, long)]
    reverse: bool,
    /// Limit results returned to "N" entries
    #[structopt(short, long)]
    number: Option<usize>,
}

struct FileInfo {
    id: i32,
    manufacturer: String,
    product: String,
    timestamp: DateTime<Local>,
    uuid: String,
}

impl TryFrom<&'_ rusqlite::Row<'_>> for FileInfo {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> Result<Self, Self::Error> {
        let (id, manufacturer, product, timestamp, uuid) = TryFrom::try_from(row)?;

        Ok(FileInfo {
            id,
            manufacturer,
            product,
            timestamp,
            uuid,
        })
    }
}

pub fn list_files_command(opts: ListFilesOpts) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db_connection()?;

    // collect all the files we are interested in
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    let mut query = QueryStringBuilder::new(
        "select id, device_manufacturer, device_product, time_created, uuid from files",
    );
    if let Some(start_date) = opts.since.as_ref() {
        query.and_where("time_created >= ?");
        params.push(start_date as &dyn rusqlite::ToSql);
    }
    if let Some(end_date) = opts.until.as_ref() {
        query.and_where("time_created < ?");
        params.push(end_date as &dyn rusqlite::ToSql);
    }
    if opts.reverse {
        query.order_by("time_created DESC");
    } else {
        query.order_by("time_created ASC");
    }
    if let Some(value) = opts.number {
        query.limit(value);
    }
    let mut stmt = conn.prepare(&query.to_string())?;
    let rows = stmt.query_map(&params, |row| FileInfo::try_from(row))?;
    let files = rows.into_iter().collect::<Result<Vec<FileInfo>>>()?;

    // grab aggregrate and lap stats
    let (agg_data, lap_data) = if opts.stat {
        (
            collect_aggregate_stats(&conn, opts.since.as_ref(), opts.until.as_ref())?,
            collect_lap_stats(&conn, opts.since.as_ref(), opts.until.as_ref())?,
        )
    } else {
        (HashMap::new(), HashMap::new())
    };

    println!("Date, Device, UUID");
    for file in files {
        println!(
            "{} {}-{} ({})",
            file.timestamp.format("%Y-%m-%d %H:%M"),
            file.manufacturer,
            file.product,
            file.uuid
        );
        if let Some(data) = agg_data.get(&file.id) {
            println!(
                "\t Distance: {:0.2} miles, Time: {:3}:{:02.0}, \
                     Pace: {:2}:{:02.0}, Heart Rate: {:0.0}bpm",
                data["total_distance"],
                data["total_time"] as i32,
                (data["total_time"] - data["total_time"].floor()) * 60.0,
                data["avg_pace"] as i32,
                (data["avg_pace"] - data["avg_pace"].floor()) * 60.0,
                data["avg_heart_rate"]
            );
        }
        if let Some(data) = lap_data.get(&file.id) {
            for (i, lap) in data.iter().enumerate() {
                println!(
                    "\t * Lap {:02} - {:0.2} miles, Time: {:3}:{:02.0}, Heart Rate: {:0.0}bpm",
                    i + 1,
                    lap["total_distance"],
                    lap["total_time"] as i32,
                    (lap["total_time"] - lap["total_time"].floor()) * 60.0,
                    lap["avg_heart_rate"]
                );
            }
        }
    }

    Ok(())
}

/// Query the record_messages table to get various values averaged across the entire run
fn collect_aggregate_stats(
    conn: &Connection,
    start_date: Option<&NaiveDate>,
    end_date: Option<&NaiveDate>,
) -> Result<HashMap<i32, HashMap<&'static str, f64>>> {
    let mut agg_data: HashMap<i32, HashMap<&'static str, f64>> = HashMap::new();
    let mut stmt;
    let mut rows = if let Some(start_date) = start_date {
        stmt = conn.prepare(
            "select max(distance) tot_dist, sum(speed)/count(speed) avg_speed,
                    sum(heart_rate)/count(heart_rate) avg_hr,
                    max(timestamp) end_time, min(timestamp) start_time,
                    file_id
                from record_messages
                inner join files on files.id = file_id
                where time_created between ? and ?
                group by files.id",
        )?;
        if let Some(end_date) = end_date {
            stmt.query(params![start_date, end_date.and_hms(23, 59, 59)])?
        } else {
            // if no end date is provided return all for the given day
            stmt.query(params![start_date, start_date.and_hms(23, 59, 59)])?
        }
    } else {
        stmt = conn.prepare(
            "select max(distance) tot_dist, sum(speed)/count(speed) avg_speed,
                    sum(heart_rate)/count(heart_rate) avg_hr,
                    max(timestamp) end_time, min(timestamp) start_time,
                    file_id
                from record_messages
                inner join files on files.id = file_id
                group by files.id",
        )?;
        stmt.query(NO_PARAMS)?
    };

    // store data after applying some unit conversions
    while let Some(row) = rows.next()? {
        let mut file_stats: HashMap<&'static str, f64> = HashMap::new();
        let total_time = row.get::<&str, DateTime<Local>>("end_time")?
            - row.get::<&str, DateTime<Local>>("start_time")?;
        file_stats.insert(
            "total_distance",
            row.get::<&str, f64>("tot_dist")? * 0.00062137,
        );
        file_stats.insert("total_time", total_time.num_seconds() as f64 / 60.0);
        file_stats.insert(
            "avg_pace",
            1.0 / (row.get::<&str, f64>("avg_speed")? * 0.00062137 * 60.0),
        );
        file_stats.insert("avg_heart_rate", row.get("avg_hr").unwrap_or(0.0));
        agg_data.insert(row.get("file_id")?, file_stats);
    }

    Ok(agg_data)
}

/// Query the record_messages table to get various values averaged across the entire run
fn collect_lap_stats(
    conn: &Connection,
    start_date: Option<&NaiveDate>,
    end_date: Option<&NaiveDate>,
) -> Result<HashMap<i32, Vec<HashMap<&'static str, f64>>>> {
    let mut lap_data: HashMap<i32, Vec<HashMap<&'static str, f64>>> = HashMap::new();
    let mut stmt;
    let mut rows = if let Some(start_date) = start_date {
        stmt = conn.prepare(
            "select average_speed, average_heart_rate, total_distance,
                    start_time, timestamp as end_time, file_id
                from lap_messages
                inner join files on files.id = file_id
                where time_created between ? and ?
                order by file_id, start_time",
        )?;
        if let Some(end_date) = end_date {
            stmt.query(params![start_date, end_date.and_hms(23, 59, 59)])?
        } else {
            // if no end date is provided return all for the given day
            stmt.query(params![start_date, start_date.and_hms(23, 59, 59)])?
        }
    } else {
        stmt = conn.prepare(
            "select average_speed, average_heart_rate, total_distance,
                    start_time, timestamp as end_time, file_id
                from lap_messages
                inner join files on files.id = file_id
                order by file_id, start_time",
        )?;
        stmt.query(NO_PARAMS)?
    };

    // store data after applying some unit conversions, we crate an empty vec here to make the
    // compiler happy since that's cleaner than extracting the first loop iteration
    let mut file_stats: Vec<HashMap<&'static str, f64>> = Vec::with_capacity(0);
    let mut curr_id: i32 = -1;
    while let Some(row) = rows.next()? {
        let mut lap_stats: HashMap<&'static str, f64> = HashMap::new();
        let total_time = row.get::<&str, DateTime<Local>>("end_time")?
            - row.get::<&str, DateTime<Local>>("start_time")?;
        let file_id: i32 = row.get("file_id")?;
        lap_stats.insert(
            "total_distance",
            row.get::<&str, f64>("total_distance")? * 0.00062137,
        );
        lap_stats.insert("total_time", total_time.num_seconds() as f64 / 60.0);
        lap_stats.insert(
            "avg_pace",
            1.0 / (row.get::<&str, f64>("average_speed")? * 0.00062137 * 60.0),
        );
        lap_stats.insert(
            "avg_heart_rate",
            row.get("average_heart_rate").unwrap_or(0.0),
        );

        // create new lap vector when file_id changes
        if curr_id != file_id {
            lap_data.insert(curr_id, file_stats);
            file_stats = Vec::new();
            curr_id = file_id;
        }
        file_stats.push(lap_stats);
    }
    // catch last iteration which gets missed by conditional
    lap_data.insert(curr_id, file_stats);

    Ok(lap_data)
}
