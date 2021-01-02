//! Access elevation data for a given GPS location using an external source
use super::db::QueryStringBuilder;
use crate::{open_db_connection, Error, Location};
use log::{error, info};
use rusqlite::params;

mod opentopodata;
pub use opentopodata::OpenTopoData;

/// trait that defines how elevation data should be added for an array of lat, long coordintes
pub trait ElevationDataSource {
    /// Updates the array of locations with elevation data
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Update elevation for a FIT file or across all data in the database
pub fn update_elevation_data<T: ElevationDataSource>(
    src: &T,
    uuid: Option<&str>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_db_connection()?;
    let tx = conn.transaction()?;

    // setup base queries
    let mut rec_query =
        QueryStringBuilder::new("select position_lat, position_long, id from record_messages");
    rec_query
        .and_where("position_lat is not null")
        .and_where("position_long is not null");
    let mut lap_query = QueryStringBuilder::new("select start_position_lat, start_position_long, end_position_lat, end_position_long, id from lap_messages");
    lap_query
        .and_where("start_position_lat is not null")
        .and_where("start_position_long is not null");
    if !overwrite {
        rec_query.and_where("elevation is null");
        lap_query.and_where("start_elevation is null");
    }

    // filter by UUID if one was defined
    let mut file_id: Option<i32> = None;
    if let Some(uuid) = uuid {
        if let Ok(id) = tx.query_row("select id from files where uuid = ?", params![uuid], |r| {
            r.get::<usize, i32>(0)
        }) {
            file_id = Some(id);
            rec_query.and_where("file_id = ?");
            lap_query.and_where("file_id = ?");
        } else {
            error!("FIT File with UUID='{}' does not exist", uuid);
            return Err(Box::new(Error::FileDoesNotExistError(uuid.to_string())));
        }
    }

    // fetch and save elevation data for record and lap messages
    let params: Vec<&dyn rusqlite::ToSql> = file_id
        .as_ref()
        .map_or(Vec::new(), |v| vec![v as &dyn rusqlite::ToSql]);
    let mut stmt = tx.prepare(&rec_query.to_string())?;
    let nrows = stmt
        .query(&params)
        .map(|rows| add_record_elevation_data(src, &tx, rows))??; // we have nested results here
    stmt.finalize()?; // appease borrow checker
    info!(
        "Set location data for {} record messages{}",
        nrows,
        uuid.map_or(String::new(), |v| format!(" in file {}", v))
    );

    let mut stmt = tx.prepare(&lap_query.to_string())?;
    let nrows = stmt
        .query(&params)
        .map(|rows| add_lap_elevation_data(src, &tx, rows))??;
    stmt.finalize()?; // appease borrow checker
    info!(
        "Set location data for {} lap messages{}",
        nrows,
        uuid.map_or(String::new(), |v| format!(" in file {}", v))
    );

    tx.commit()?;
    Ok(())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_record_elevation_data<T: ElevationDataSource>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut locations: Vec<Location> = Vec::new();
    let mut record_ids: Vec<i32> = Vec::new();
    while let Some(row) = rows.next()? {
        locations.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
        record_ids.push(row.get(2)?);
    }
    src.request_elevation_data(&mut locations)?;

    let mut stmt = tx.prepare_cached("update record_messages set elevation = ? where id = ?")?;
    for (loc, rec_id) in locations.iter().zip(record_ids) {
        stmt.execute(params![loc.elevation().map(|v| v as f64), rec_id])?;
    }

    Ok(locations.len())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_lap_elevation_data<T: ElevationDataSource>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut st_locations: Vec<Location> = Vec::new();
    let mut en_locations: Vec<Location> = Vec::new();
    let mut record_ids: Vec<i32> = Vec::new();
    while let Some(row) = rows.next()? {
        st_locations.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
        en_locations.push(Location::from_fit_coordinates(row.get(2)?, row.get(3)?));
        record_ids.push(row.get(4)?);
    }
    src.request_elevation_data(&mut st_locations)?;
    src.request_elevation_data(&mut en_locations)?;

    let mut stmt = tx.prepare_cached(
        "update lap_messages set start_elevation = ?, end_elevation = ? where id = ?",
    )?;
    for ((st_loc, en_loc), rec_id) in st_locations.iter().zip(en_locations).zip(record_ids) {
        stmt.execute(params![
            st_loc.elevation().map(|v| v as f64),
            en_loc.elevation().map(|v| v as f64),
            rec_id
        ])?;
    }

    Ok(st_locations.len())
}
