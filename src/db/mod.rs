//! Database utility functions and the schema definition
use dirs;
use log::debug;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

mod schema;
pub use schema::create_database;

static DATABASE_NAME: &str = "garmin-run-tracker.db";

pub fn open_db_connection() -> Result<Connection> {
    let db = db_path();
    debug!("Connected to local database located at: {:?}", db);
    Connection::open(&db)
}

pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or(PathBuf::new())
        .join(DATABASE_NAME)
}
