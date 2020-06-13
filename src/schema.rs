use dirs;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

static DATABASE_NAME: &str = "garmin-run-tracker.db";

/// Create the database and required tables
pub fn create_database() -> Result<()> {
    let db_path = dirs::data_dir().unwrap_or(PathBuf::new()).join(DATABASE_NAME);
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "create table files (
            type           text not null,
            manufacturer   text,
            product        text,
            time_created   datetime not null,
            serial_number  integer not null,
            id             integer primary key
        )",
        params![],
    )?;

    conn.execute(
        "create table record_messages (
            position_lat  integer,
            position_long integer,
            speed         float,
            distance      float,
            heart_rate    integer,
            timestamp     datetime not null.
            file_id       integer not null,
            id            integer primary key
        )",
        params![],
    )?;

    conn.execute(
        "create table lap_messages (
            start_position_lat  integer,
            start_position_long integer,
            end_position_lat    integer,
            end_position_long   integer,
            average_speed       float,
            average_heart_rate  integer,
            total_calories      integer,
            total_distance      float,
            start_time          datetime not null,
            timestamp           datetime not null.
            file_id             integer not null,
            id                  integer primary key
        )",
        params![],
    )?;

    Ok(())
}
