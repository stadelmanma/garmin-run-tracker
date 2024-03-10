//! Define the list-files subcommand
use super::parse_date;
use crate::db::{new_file_info_query, open_db_connection};
use crate::FileInfo;
use chrono::{DateTime, Local, NaiveDate};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Result};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::rc::Rc;
use structopt::StructOpt;

/// List all files in the local database
#[derive(Debug, StructOpt)]
pub struct ListFilesOpts {
    /// Hide per file statistics and only show date, device and UUID of entries
    #[structopt(short, long)]
    short: bool,
    /// List files after the specified date (YYYY-MM-DD format)
    #[structopt(short="-S", long, parse(try_from_str = parse_date))]
    since: Option<NaiveDate>,
    /// List files before the specified date (YYYY-MM-DD format)
    #[structopt(short="-U", long, parse(try_from_str = parse_date))]
    until: Option<NaiveDate>,
    /// Reverse file ordering to be old -> new
    #[structopt(short, long)]
    reverse: bool,
    /// Limit results returned to "N" entries ()
    #[structopt(short, long)]
    number: Option<usize>,
}

pub fn list_files_command(opts: ListFilesOpts) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db_connection()?;

    // collect all the files we are interested in
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    let mut query = new_file_info_query();
    if let Some(start_date) = opts.since.as_ref() {
        query.and_where("time_created >= ?");
        params.push(start_date as &dyn rusqlite::ToSql);
    }
    if let Some(end_date) = opts.until.as_ref() {
        query.and_where("time_created < ?");
        params.push(end_date as &dyn rusqlite::ToSql);
    }
    if opts.reverse {
        query.order_by("time_created ASC");
    } else {
        query.order_by("time_created DESC");
    }
    if let Some(value) = opts.number {
        query.limit(value);
    }
    let mut stmt = conn.prepare(&query.to_string())?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |r| FileInfo::try_from(r))?;
    let mut file_ids = Vec::new();
    let mut files = Vec::new();
    for r in rows {
        let r = r?;
        file_ids.push(Value::from(r.id));
        files.push(r);
    }
    let values: Rc<Vec<Value>> = Rc::new(file_ids); // usage of select from rarray needs an Rc

    // grab aggregrate and lap stats
    if opts.short {
        let agg_data = collect_aggregate_stats(&conn, Rc::clone(&values))?;
        short_output(&files, agg_data);
    } else {
        let agg_data = collect_aggregate_stats(&conn, Rc::clone(&values))?;
        let lap_data = collect_lap_stats(&conn, Rc::clone(&values))?;
        long_output(&files, agg_data, lap_data);
    };

    Ok(())
}

fn short_output(files: &[FileInfo], agg_data: HashMap<u32, HashMap<&'static str, f64>>) {
    println!("Date\tDistance[mi]\tPace[mi/min]\tUUID");
    for file in files {
        match file.id.map(|id| agg_data.get(&id)).flatten() {
            Some(data) => {
                println!(
                    "{:10}\t{:0.2}\t{:2}:{:02.0}\t({})",
                    file.timestamp.format("%Y-%m-%d"),
                    data["total_distance"],
                    data["avg_pace"] as i32,
                    (data["avg_pace"] - data["avg_pace"].floor()) * 60.0,
                    file.uuid
                );
            }
            None => {
                println!(
                    "{} {}-{} ({})",
                    file.timestamp.format("%Y-%m-%d %H:%M"),
                    file.manufacturer,
                    file.product,
                    file.uuid
                );
            }
        }
    }
}

fn long_output(
    files: &[FileInfo],
    agg_data: HashMap<u32, HashMap<&'static str, f64>>,
    lap_data: HashMap<u32, Vec<HashMap<&'static str, f64>>>,
) {
    println!("Date, Device, UUID");
    for file in files {
        println!(
            "{} ({}-{} {})",
            file.timestamp.format("%Y-%m-%d %H:%M"),
            file.manufacturer,
            file.product,
            file.uuid
        );
        let file_id = if let Some(val) = file.id {
            val
        } else {
            continue;
        };
        if let Some(data) = agg_data.get(&file_id) {
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
        if let Some(data) = lap_data.get(&file_id) {
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
}

/// Query the record_messages table to get various values averaged across the entire run
fn collect_aggregate_stats(
    conn: &Connection,
    file_ids: Rc<Vec<Value>>,
) -> Result<HashMap<u32, HashMap<&'static str, f64>>> {
    let mut agg_data: HashMap<u32, HashMap<&'static str, f64>> = HashMap::new();
    let mut stmt = conn.prepare(
        "select max(distance) tot_dist, sum(speed)/count(speed) avg_speed,
                    sum(heart_rate)/count(heart_rate) avg_hr,
                    max(timestamp) end_time, min(timestamp) start_time,
                    file_id
                from record_messages
                where file_id in (select value from rarray(?))
                group by file_id",
    )?;
    let mut rows = stmt.query(params![file_ids])?;

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
    file_ids: Rc<Vec<Value>>,
) -> Result<HashMap<u32, Vec<HashMap<&'static str, f64>>>> {
    let mut lap_data: HashMap<u32, Vec<HashMap<&'static str, f64>>> = HashMap::new();
    let mut stmt = conn.prepare(
        "select average_speed, average_heart_rate, total_distance,
                    start_time, timestamp as end_time, file_id
                from lap_messages
                where file_id in (select value from rarray(?))
                order by file_id, start_time",
    )?;
    let mut rows = stmt.query(params![file_ids])?;

    // store data after applying some unit conversions, we crate an empty vec here to make the
    // compiler happy since that's cleaner than extracting the first loop iteration
    let mut file_stats: Vec<HashMap<&'static str, f64>> = Vec::with_capacity(0);
    let mut curr_id = 4294967295u32;
    while let Some(row) = rows.next()? {
        let mut lap_stats: HashMap<&'static str, f64> = HashMap::new();
        let total_time = row.get::<&str, DateTime<Local>>("end_time")?
            - row.get::<&str, DateTime<Local>>("start_time")?;
        let file_id: u32 = row.get("file_id")?;
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
