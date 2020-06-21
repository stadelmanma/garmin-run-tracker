use chrono::Utc;
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use log::{debug, error, trace};
use rusqlite::types::ToSqlOutput;
use rusqlite::{NO_PARAMS, params, ToSql};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::ops::Deref;

mod elevation;
pub use elevation::{Location, request_elevation_data};
mod schema;
pub use schema::{create_database, open_db_connection};

/// General error type for the crate
#[derive(Debug)]
enum Error {
    ArrayConversionError,
    DuplicateFileError,
    ElevationRequestError(reqwest::StatusCode, String),
    FileDoesNotExistError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ArrayConversionError => write!(f, "Cannot convert Value:Array into a SQL parameter"),
            Error::DuplicateFileError => write!(f, "Attempted to import a file already in the database"),
            Error::ElevationRequestError(code, msg) => write!(f, "Elevation data request failed with code: {} - {}", code, msg),
            Error::FileDoesNotExistError(uuid) => write!(f, "FIT File with UUID='{}' does not exist", uuid),
        }
    }
}

impl std::error::Error for Error {}

/// Acts as a pointer to a Value variant that can be used in parameterized sql statements
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct SqlValue<'a>(&'a Value);

impl SqlValue<'_> {
    /// Wrap a reference to a Value parsed from a FIT file
    pub fn new(value: &Value) -> SqlValue {
        SqlValue(value)
    }
}

impl Deref for SqlValue<'_> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SqlValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ToSql for SqlValue<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self.0 {
            Value::Timestamp(val) => Ok(ToSqlOutput::from(val.with_timezone(&Utc).to_rfc3339())),
            Value::Byte(val) => Ok(ToSqlOutput::from(*val)),
            Value::Enum(val) => Ok(ToSqlOutput::from(*val)),
            Value::SInt8(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt8(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt8z(val) => Ok(ToSqlOutput::from(*val)),
            Value::SInt16(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt16(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt16z(val) => Ok(ToSqlOutput::from(*val)),
            Value::SInt32(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt32(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt32z(val) => Ok(ToSqlOutput::from(*val)),
            Value::SInt64(val) => Ok(ToSqlOutput::from(*val)),
            Value::UInt64(val) => Ok(ToSqlOutput::from(*val as i64)),
            Value::UInt64z(val) => Ok(ToSqlOutput::from(*val as i64)),
            Value::Float32(val) => Ok(ToSqlOutput::from(*val as f64)),
            Value::Float64(val) => Ok(ToSqlOutput::from(*val)),
            Value::String(val) => Ok(ToSqlOutput::Borrowed(val.as_bytes().into())),
            Value::Array(_) => Err(rusqlite::Error::ToSqlConversionFailure(
                Box::new(Error::ArrayConversionError))),
        }
    }
}

/// Import raw fit file data into the local database
pub fn import_fit_data<T: Read>(fp: &mut T) -> Result<String, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    fp.read_to_end(&mut data)?;

    // hash the fit file for deduplication purposes
    let uuid = generate_uuid(&data);
    trace!("UUID hash of file: {}", uuid);

    // connect to database and see if the UUID is aleady present before parsing
    let mut conn = open_db_connection()?;
    if let Ok(()) = conn.query_row("select id from files where uuid = ?", params![uuid], |_| Ok(())) {
        error!("Attempted to import a file already in the database, UUID: {}", uuid);
        return Err(Box::new(Error::DuplicateFileError));
    }

    // parse the fit file
    let messages = fitparser::from_bytes(&data)?;
    trace!("Parsed FIT file and found {} messages", messages.len());

    // loop over messages, the file_id message starts a new FIT file and any records appearing
    // before it are disregarded.
    let tx = conn.transaction()?;
    let mut file_rec_id = None;
    for mesg in messages {
        let data = create_fit_data_map(&mesg);
        match mesg.kind() {
            MesgNum::FileId => {
                // insert new file record into db and set file_rec_id to the row id
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
                file_rec_id = Some(tx.last_insert_rowid());
                debug!("Processed and stored file_id message with data: {:?}", data)
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
                debug!("Processed and stored lap message with data: {:?}", data)
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
                debug!("Processed and stored record message with data: {:?}", data)
            }
            _ => trace!("Skipped {} message with data: {:?}", mesg.kind(), data),
        }
    }
    // commit transaction to store data imported from file and then fetch elevation data
    tx.commit()?;
    if let Err(e) = update_elevation_data(Some(&uuid)) {
        error!("Could not add in elevation data from the API for file with UUID='{}'", uuid);
        error!("{}", e)
    }

    Ok(uuid)
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
pub fn create_fit_data_map<'a>(mesg: &'a FitDataRecord) -> HashMap<&'a str, SqlValue> {
    HashMap::from_iter(
        mesg.fields()
            .iter()
            .map(|f| (f.name(), SqlValue::new(f.value()))),
    )
}

/// Update elevation for a FIT file or across all data in the database
pub fn update_elevation_data(uuid: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_db_connection()?;
    let tx = conn.transaction()?;

    // first add elevation data for the record messages
    match uuid {
        Some(uuid) => {
            if let Ok(id) = tx.query_row("select id from files where uuid = ?", params![uuid], |r| r.get::<usize, i32>(0)) {
                let mut stmt = tx.prepare("select position_lat, position_long, id from record_messages where
                                             file_id = ? and
                                             position_lat is not null and
                                             position_long is not null")?;
                let rows = stmt.query(params![id])?;
                request_and_update_elevation_data(&tx, "record_messages", rows)?;
            }
            else {
                error!("FIT File with UUID='{}' does not exist", uuid);
                return Err(Box::new(Error::FileDoesNotExistError(uuid.to_string())));
            }
        },
        None => {
            let mut stmt = tx.prepare("select position_lat, position_long, id from record_messages where
                                         position_lat is not null and
                                         position_long is not null and
                                         elevation is null")?;
            let rows = stmt.query(NO_PARAMS)?;
            request_and_update_elevation_data(&tx, "record_messages", rows)?;
        }
    };

    // todo lap messages

    tx.commit()?;
    Ok(())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn request_and_update_elevation_data(tx: &rusqlite::Transaction, table: &'static str, mut rows: rusqlite::Rows) -> Result<(), Box<dyn std::error::Error>> {
    let mut locations: Vec<Location> = Vec::new();
    let mut record_ids: Vec<i32> = Vec::new();
    while let Some(row) = rows.next()? {
        locations.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
        record_ids.push(row.get(2)?);
    }
    request_elevation_data(&mut locations)?;

    let stmt = format!("update {} set elevation = ? where id = ?", table);
    let mut stmt = tx.prepare_cached(&stmt)?;
    for (loc, rec_id) in locations.iter().zip(record_ids) {
        stmt.execute(params![loc.elevation().map(|v| v as f64), rec_id])?;
    }

    Ok(())
}
