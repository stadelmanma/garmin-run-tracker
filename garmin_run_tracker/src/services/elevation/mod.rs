//! Access elevation data for a given GPS location using an external source
use crate::config::{FromServiceConfig, ServiceConfig};
use crate::db::QueryStringBuilder;
use crate::gps::Location;
use crate::Error;
use log::{info, warn};
use rusqlite::{params, params_from_iter, Transaction};

mod opentopodata;
pub use opentopodata::OpenTopoData;
mod mapquest_elevation_api;
pub use mapquest_elevation_api::MapquestElevationApi;

/// trait that defines how elevation data should be added for an array of lat, long coordintes
pub trait ElevationDataSource {
    /// Updates the array of locations with elevation data
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>>;
}

pub fn new_elevation_handler(
    config: &ServiceConfig,
) -> Result<Box<dyn ElevationDataSource>, Error> {
    match config.handler() {
        "opentopodata" => Ok(Box::new(OpenTopoData::from_config(config)?)),
        "mapquest" => Ok(Box::new(MapquestElevationApi::from_config(config)?)),
        _ => Err(Error::UnknownServiceHandler(format!(
            "no elevation handler exists for: {}",
            config.handler()
        ))),
    }
}

/// Update elevation for a FIT file or across all data in the database
pub fn update_elevation_data<T: ElevationDataSource + ?Sized>(
    tx: &Transaction,
    src: &T,
    file_id: Option<u32>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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
    if file_id.is_none() || !overwrite {
        rec_query.and_where("elevation is null");
        lap_query.and_where("start_elevation is null");
    }
    if file_id.is_some() {
        rec_query.and_where("file_id = ?");
        lap_query.and_where("file_id = ?");
    }
    if overwrite && file_id.is_none() {
        warn!("Refusing to overwrite all elevation data, specify individual files instead");
    }

    // fetch and save elevation data for record and lap messages
    let params: Vec<&dyn rusqlite::ToSql> = file_id
        .as_ref()
        .map_or(Vec::new(), |v| vec![v as &dyn rusqlite::ToSql]);
    let mut stmt = tx.prepare(&rec_query.to_string())?;
    let (nset, nrows) = stmt
        .query(params_from_iter(params.iter()))
        .map(|rows| add_record_elevation_data(src, &tx, rows))??; // we have nested results here
    stmt.finalize()?; // appease borrow checker
    info!("Set location data for {}/{} record messages", nset, nrows,);

    let mut stmt = tx.prepare(&lap_query.to_string())?;
    let (nset, nrows) = stmt
        .query(params_from_iter(params.iter()))
        .map(|rows| add_lap_elevation_data(src, &tx, rows))??;
    stmt.finalize()?; // appease borrow checker
    info!("Set location data for {}/{} lap messages", nset, nrows,);

    Ok(())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_record_elevation_data<T: ElevationDataSource + ?Sized>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
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

    Ok((
        locations.iter().filter(|l| l.elevation().is_some()).count(),
        locations.len(),
    ))
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_lap_elevation_data<T: ElevationDataSource + ?Sized>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
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

    Ok((
        st_locations
            .iter()
            .filter(|l| l.elevation().is_some())
            .count(),
        st_locations.len(),
    ))
}
