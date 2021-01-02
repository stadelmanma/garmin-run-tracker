//! Access elevation data for a given GPS location using an external source
use crate::{open_db_connection, Error, Location};
use log::error;
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
    uuid: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = open_db_connection()?;
    let tx = conn.transaction()?;

    if let Ok(id) = tx.query_row("select id from files where uuid = ?", params![uuid], |r| {
        r.get::<usize, i32>(0)
    }) {
        // first add elevation data for the record messages
        let mut stmt = tx.prepare(
            "select position_lat, position_long, id from record_messages where
                                     file_id = ? and
                                     position_lat is not null and
                                     position_long is not null",
        )?;
        let rows = stmt.query(params![id])?;
        add_record_elevation_data(src, &tx, rows)?;

        // next add in elevation data for lap messages
        let mut stmt = tx.prepare("select start_position_lat, start_position_long, end_position_lat, end_position_long, id from lap_messages where
                                     file_id = ? and
                                     start_position_lat is not null and
                                     start_position_long is not null")?;
        let rows = stmt.query(params![id])?;
        add_lap_elevation_data(src, &tx, rows)?;
    } else {
        error!("FIT File with UUID='{}' does not exist", uuid);
        return Err(Box::new(Error::FileDoesNotExistError(uuid.to_string())));
    }

    tx.commit()?;
    Ok(())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_record_elevation_data<T: ElevationDataSource>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut locations: Vec<Location> = Vec::new();
    let mut record_ids: Vec<i32> = Vec::new();
    while let Some(row) = rows.next()? {
        locations.push(Location::from_fit_coordinates(row.get(0)?, row.get(1)?));
        record_ids.push(row.get(2)?);
    }
    src.request_elevation_data(&mut locations)?;

    let stmt = format!("update record_messages set elevation = ? where id = ?");
    let mut stmt = tx.prepare_cached(&stmt)?;
    for (loc, rec_id) in locations.iter().zip(record_ids) {
        stmt.execute(params![loc.elevation().map(|v| v as f64), rec_id])?;
    }

    Ok(())
}

/// Updates a set of rows with elevation data by querying the elevation API and then passing that
/// data back into the database
fn add_lap_elevation_data<T: ElevationDataSource>(
    src: &T,
    tx: &rusqlite::Transaction,
    mut rows: rusqlite::Rows,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let stmt =
        format!("update lap_messages set start_elevation = ?, end_elevation =? where id = ?");
    let mut stmt = tx.prepare_cached(&stmt)?;
    for ((st_loc, en_loc), rec_id) in st_locations.iter().zip(en_locations).zip(record_ids) {
        stmt.execute(params![
            st_loc.elevation().map(|v| v as f64),
            en_loc.elevation().map(|v| v as f64),
            rec_id
        ])?;
    }

    Ok(())
}
