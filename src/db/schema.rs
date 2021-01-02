use super::{db_path, open_db_connection};
use log::debug;
use rusqlite::{params, Result};

/// Create the database and required tables
pub fn create_database() -> Result<()> {
    let db = db_path();
    if db.exists() {
        debug!(
            "Skipping database initialization, pre-existing database found at {:?}",
            db
        );
        return Ok(());
    }

    let mut conn = open_db_connection()?;
    let tx = conn.transaction()?;
    tx.execute(
        "create table if not exists files (
            type                  text not null,
            device_manufacturer   text,
            device_product        text,
            device_serial_number  integer not null,
            time_created          datetime not null,
            uuid                  text not null, -- used for deduplication
            id                    integer primary key
        )",
        params![],
    )?;

    tx.execute(
        "create table if not exists record_messages (
            position_lat  integer,
            position_long integer,
            speed         float,
            distance      float,
            elevation     float,
            heart_rate    integer,
            timestamp     datetime not null,
            file_id       integer not null,
            id            integer primary key
        )",
        params![],
    )?;

    tx.execute(
        "create table if not exists lap_messages (
            start_position_lat  integer,
            start_position_long integer,
            start_elevation     float,
            end_position_lat    integer,
            end_position_long   integer,
            end_elevation       float,
            average_speed       float,
            average_heart_rate  integer,
            total_calories      integer,
            total_distance      float,
            start_time          datetime not null,
            timestamp           datetime not null,
            file_id             integer not null,
            id                  integer primary key
        )",
        params![],
    )?;

    tx.commit()?;
    debug!("Completed database initialization");
    Ok(())
}
