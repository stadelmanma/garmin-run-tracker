use chrono::Utc;
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use rusqlite::types::ToSqlOutput;
use rusqlite::{params, Result, ToSql};
use std::collections::HashMap;
use std::fmt;
use std::iter::FromIterator;
use std::ops::Deref;

mod schema;
pub use schema::{create_database, open_db_connection};

/// General error type for the crate
#[derive(Debug)]
struct Error {
    message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
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

impl ToSql for SqlValue<'_> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
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
            Value::Array(_) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(Error {
                message: "Cannot convert Value:Array into a SQL parameter",
            }))),
        }
    }
}

/// Import parsed fit file data into the local database
pub fn import_fit_data(messages: &[FitDataRecord]) -> Result<()> {
    let mut conn = open_db_connection()?;
    let tx = conn.transaction()?;

    // loop over messages, the file_id message starts a new FIT file and any records appearing
    // before it are disregarded.
    let mut file_rec_id = None;
    for mesg in messages {
        let data = create_fit_data_map(mesg);
        match mesg.kind() {
            MesgNum::FileId => {
                // insert new file record into db and set file_rec_id to the row id
                let mut stmt = tx.prepare_cached(
                    "insert into files (type, manufacturer, product, time_created, serial_number)
                     values (?1, ?2, ?3, ?4, ?5)",
                )?;
                stmt.execute(params![
                    data.get("type"),
                    data.get("manufacturer"),
                    data.get("garmin_product"),
                    data.get("time_created"),
                    data.get("serial_number")
                ])?;
                file_rec_id = Some(tx.last_insert_rowid());
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
            }
            _ => {
                // todo in debug logging mode output that this message was ignored
            }
        }
    }

    tx.commit()
}

/// Build a hash map of field references that can be acessed by field name
pub fn create_fit_data_map<'a>(mesg: &'a FitDataRecord) -> HashMap<&'a str, SqlValue> {
    HashMap::from_iter(
        mesg.fields()
            .iter()
            .map(|f| (f.name(), SqlValue::new(f.value()))),
    )
}
