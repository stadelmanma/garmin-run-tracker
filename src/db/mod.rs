//! Database utility functions and the schema definition
use dirs;
use log::debug;
use rusqlite::{Connection, Result};
use std::fmt;
use std::path::PathBuf;

mod schema;
pub use schema::create_database;

static DATABASE_NAME: &str = "garmin-run-tracker.db";

// very basic declarative query constructor
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

pub fn open_db_connection() -> Result<Connection> {
    let db = db_path();
    let conn = Connection::open(&db)?;
    rusqlite::vtab::array::load_module(&conn)?;
    debug!("Connected to local database located at: {:?}", db);
    Ok(conn)
}

pub fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or(PathBuf::new())
        .join(DATABASE_NAME)
}
