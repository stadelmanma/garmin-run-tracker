use chrono::{DateTime, Local, TimeZone};
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use log::trace;
use rusqlite::{params, Transaction};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::ops::Deref;
use std::path::PathBuf;

pub mod cli;
pub mod config;
pub use config::Config;
mod db;
pub use db::{create_database, open_db_connection};
use db::{find_file_by_uuid, SqlValue};
mod error;
pub use error::Error;
pub mod gps;
pub mod services;

static DIRECTORY_NAME: &str = "garmin-run-tracker";

/// Contains basic information about a single FIT file, if the file is chained this struct
/// will get updated to the last file in the chain.
#[derive(Debug)]
pub struct FileInfo {
    id: Option<u32>,
    manufacturer: String,
    product: String,
    serial_number: u32,
    timestamp: DateTime<Local>,
    uuid: String,
}

impl FileInfo {
    /// Return row_id in the database for this file
    pub fn id(&self) -> Option<u32> {
        self.id
    }

    /// Return manufacturer field if set
    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    /// Return product field if set
    pub fn product(&self) -> &str {
        &self.product
    }

    /// Return serial number of the device used to crated the file
    pub fn serial_number(&self) -> u32 {
        self.serial_number
    }

    pub fn timestamp(&self) -> &DateTime<Local> {
        &self.timestamp
    }

    /// Return UUID generated from this file's byte stream
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or(PathBuf::new())
        .join(DIRECTORY_NAME)
}

pub fn devices_dir() -> PathBuf {
    data_dir().join("devices")
}

pub fn load_config() -> Result<Config, Error> {
    let file = data_dir().join("config.yml");
    let mut fp = File::open(&file)?;
    Config::load(&mut fp).map_err(Error::from)
}

/// Import raw fit file data into the local database
pub fn import_fit_data<T: Read>(fp: &mut T, tx: &Transaction) -> Result<FileInfo, Error> {
    let mut data = Vec::new();
    fp.read_to_end(&mut data)?;

    // hash the fit file for deduplication purposes
    let uuid = generate_uuid(&data);
    trace!("UUID hash of file: {}", uuid);

    // connect to database and see if the UUID is aleady present before parsing
    if let Ok(_) = find_file_by_uuid(tx, &uuid) {
        return Err(Error::DuplicateFileError(uuid));
    }

    // parse the fit file
    let messages = fitparser::from_bytes(&data)?;
    trace!("Parsed FIT file and found {} messages", messages.len());

    // loop over messages, the file_id message starts a new FIT file and any records appearing
    // before it are disregarded.
    let mut file_rec_id = None;
    let mut file_info = None;
    for mesg in messages {
        let data = create_fit_data_map(&mesg);
        match mesg.kind() {
            MesgNum::FileId => {
                // insert new file record into db and set file_rec_id to the row id
                // this message must exist before any others since there is a NULL constraint
                // on the file_id column in the lap and record tables
                let mut stmt = tx.prepare_cached(
                    "insert into files (type,
                                        device_manufacturer,
                                        device_product,
                                        device_serial_number,
                                        time_created,
                                        uuid)
                     values (?1, ?2, ?3, ?4, ?5, ?6)",
                )?;
                stmt.execute(params![
                    data.get("type"),
                    data.get("manufacturer"),
                    data.get("garmin_product"),
                    data.get("serial_number"),
                    data.get("time_created"),
                    uuid,
                ])?;
                let timestamp = data.get("time_created").map_or(Local.timestamp(0, 0), |v| {
                    if let Value::Timestamp(v) = v.deref() {
                        v.clone()
                    } else {
                        Local.timestamp(0, 0)
                    }
                });
                let serial_number = data
                    .get("serial_number")
                    .map_or(Ok(-1i64), |v| v.deref().clone().try_into())?;
                file_rec_id = Some(tx.last_insert_rowid() as u32);
                file_info = Some(FileInfo {
                    id: file_rec_id,
                    manufacturer: data
                        .get("manufacturer")
                        .map_or(String::new(), |v| v.to_string()),
                    product: data
                        .get("garmin_product")
                        .map_or(String::new(), |v| v.to_string()),
                    serial_number: serial_number as u32,
                    timestamp,
                    uuid: uuid.clone(),
                });
                trace!("Processed and stored file_id message with data: {:?}", data)
            }
            MesgNum::Lap => {
                // store lap mesage
                let mut stmt = tx.prepare_cached(
                    "insert into lap_messages
                     (start_position_lat,
                      start_position_long,
                      end_position_lat,
                      end_position_long,
                      average_speed,
                      average_heart_rate,
                      total_calories,
                      total_distance,
                      start_time,
                      timestamp,
                      file_id)
                     values (?1, ?2, ?3, ?4, ?5,?6, ?7, ?8, ?9, ?10, ?11)",
                )?;
                stmt.execute(params![
                    data.get("start_position_lat"),
                    data.get("start_position_long"),
                    data.get("end_position_lat"),
                    data.get("end_position_long"),
                    data.get("enhanced_avg_speed"),
                    data.get("avg_heart_rate"),
                    data.get("total_calories"),
                    data.get("total_distance"),
                    data.get("start_time"),
                    data.get("timestamp"),
                    file_rec_id
                ])?;
                trace!("Processed and stored lap message with data: {:?}", data)
            }
            MesgNum::Record => {
                // store record mesage
                let mut stmt = tx.prepare_cached(
                    "insert into record_messages
                     (position_lat,
                      position_long,
                      speed,
                      distance,
                      heart_rate,
                      timestamp,
                      file_id)
                     values (?1, ?2, ?3, ?4, ?5,?6, ?7)",
                )?;
                stmt.execute(params![
                    data.get("position_lat"),
                    data.get("position_long"),
                    data.get("enhanced_speed"),
                    data.get("distance"),
                    data.get("heart_rate"),
                    data.get("timestamp"),
                    file_rec_id
                ])?;
                trace!("Processed and stored record message with data: {:?}", data)
            }
            _ => trace!("Skipped {} message with data: {:?}", mesg.kind(), data),
        }
    }
    file_info.ok_or(Error::FileIdMessageNotFound(uuid))
}

/// Create a UUID by taking the SHA256 hash of the data and then converting it to UUID4 format
fn generate_uuid(data: &[u8]) -> String {
    // Create a SHA256 hash from the data
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut result = hasher.finalize();

    // set version and variant bits
    result[6] = (result[6] & 0b00001111) | 0b01001111;
    result[10] = (result[10] & 0b00111111) | 0b10111111;

    // encode entire byte array and then truncate result and add grouping dashes
    let mut uuid = hex::encode(result);
    uuid.truncate(32);
    uuid.insert(20, '-');
    uuid.insert(16, '-');
    uuid.insert(12, '-');
    uuid.insert(8, '-');

    uuid
}

/// Build a hash map of field references that can be acessed by field name
fn create_fit_data_map<'a>(mesg: &'a FitDataRecord) -> HashMap<&'a str, SqlValue> {
    HashMap::from_iter(
        mesg.fields()
            .iter()
            .map(|f| (f.name(), SqlValue::new(f.value()))),
    )
}
