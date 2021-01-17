//! Database utility functions and the schema definition
use crate::{data_dir, Error, FileInfo};
use chrono::Utc;
use fitparser::Value;
use log::debug;
use rusqlite::types::ToSqlOutput;
use rusqlite::{params, Connection, Result, ToSql, Transaction};
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

mod schema;
pub use schema::create_database;

static DATABASE_NAME: &str = "garmin-run-tracker.db";

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
            // treating this as bytes causes it to be a Blob on query, even though the column is text
            Value::String(val) => Ok(ToSqlOutput::Owned(rusqlite::types::Value::Text(
                val.to_string(),
            ))),
            Value::Array(_) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                Error::ArrayConversionError,
            ))),
        }
    }
}

/// very basic declarative query constructor
pub struct QueryStringBuilder<'q> {
    base_query: &'q str,
    where_clauses: Vec<&'q str>,
    order_by: Vec<&'q str>,
    limit: Option<usize>,
}

impl<'q> QueryStringBuilder<'q> {
    pub fn new(base_query: &'q str) -> Self {
        QueryStringBuilder {
            base_query,
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
        }
    }

    pub fn and_where(&mut self, clause: &'q str) -> &mut Self {
        self.where_clauses.push(clause);
        self
    }

    pub fn order_by(&mut self, clause: &'q str) -> &mut Self {
        self.order_by.push(clause);
        self
    }

    pub fn limit(&mut self, value: usize) -> &mut Self {
        self.limit = Some(value);
        self
    }
}

impl<'q> fmt::Display for QueryStringBuilder<'q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let where_clause = if self.where_clauses.is_empty() {
            String::new()
        } else {
            let base = format!(" where {}", self.where_clauses[0]);
            self.where_clauses[1..]
                .iter()
                .fold(base, |b, c| format!("{} and {}", b, c))
        };
        let order_by = if self.order_by.is_empty() {
            String::new()
        } else {
            let base = format!(" order by {}", self.order_by[0]);
            self.order_by[1..]
                .iter()
                .fold(base, |b, c| format!("{}, {}", b, c))
        };
        let limit = if let Some(value) = self.limit {
            format!(" limit {}", value)
        } else {
            String::new()
        };
        write!(
            f,
            "{}{}{}{}",
            self.base_query, where_clause, order_by, limit
        )
    }
}

impl TryFrom<&'_ rusqlite::Row<'_>> for FileInfo {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> Result<Self, Self::Error> {
        let (id, manufacturer, product, serial_number, timestamp, uuid) = TryFrom::try_from(row)?;

        Ok(FileInfo {
            id,
            manufacturer,
            product,
            serial_number,
            timestamp,
            uuid,
        })
    }
}

/// Return the path to the database file
pub fn db_path() -> PathBuf {
    data_dir().join(DATABASE_NAME)
}

/// Open a fresh connection to the application database
pub fn open_db_connection() -> Result<Connection> {
    let db = db_path();
    let conn = Connection::open(&db)?;
    rusqlite::vtab::array::load_module(&conn)?;
    debug!("Connected to local database located at: {:?}", db);
    Ok(conn)
}

/// Return a QueryStringBuilder with the correct columns and column ordering to allow try_from
/// to be used to convert the rusqlite::Row into a FileInfo via FileInfo::try_from(row)
pub fn new_file_info_query() -> QueryStringBuilder<'static> {
    QueryStringBuilder::new(
        "select id, device_manufacturer, device_product, device_serial_number, time_created, uuid from files",
    )
}

/// Attempt to locate a specific file by it's UUID
pub fn find_file_by_uuid(tx: &Transaction, uuid: &str) -> Result<FileInfo, Error> {
    let mut query = new_file_info_query();
    query.and_where("uuid = ?");
    tx.query_row(&query.to_string(), params![uuid], |row| {
        FileInfo::try_from(row)
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Error::FileDoesNotExistError(uuid.to_string()),
        _ => Error::from(e),
    })
}
