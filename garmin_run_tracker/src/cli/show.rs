//! Define show subcommand
use crate::config::Config;
use crate::db::{find_file_by_uuid, open_db_connection};
use crate::services::visualization::plotting::{DataSeries, Plot};
use rusqlite::{params, Result};
//use std::fs::File;
//use std::io::{self, Write};
//use std::path::PathBuf;
use structopt::StructOpt;

/// Show file stats and plot running data
#[derive(Debug, StructOpt)]
pub struct ShowOpts {
    /// Full or partial UUID of file we want to generate route image for (use list-files command
    /// to see UUIDs). The special identifier :last will return the most recent file import.
    #[structopt(name = "FILE_UUID", default_value = ":last")]
    uuid: String,
}

pub fn show_command(config: Config, opts: ShowOpts) -> Result<(), Box<dyn std::error::Error>> {
    let plotter = config.get_plotting_visualization_handler()?;
    let conn = open_db_connection()?;

    // locate file_id from uuid
    let file_id = match find_file_by_uuid(&conn, &opts.uuid) {
        Ok(info) => info.id,
        Err(e) => return Err(Box::new(e)),
    };

    // fetch per-record values from messages for plotting
    let mut stmt = conn.prepare(
        "select distance, speed, elevation, heart_rate from record_messages where
                                 file_id = ?
                                 order by timestamp",
    )?;
    let mut rows = stmt.query(params![file_id])?;
    let mut distance: Vec<f64> = Vec::new();
    let mut speed: Vec<f64> = Vec::new();
    let mut elevation: Vec<f64> = Vec::new();
    let mut heart_rate: Vec<f64> = Vec::new();
    while let Some(row) = rows.next()? {
        distance.push(row.get::<usize, f64>(0)? * 0.0006213712);
        if let Ok(v) = row.get::<usize, f64>(1) {
            if v != 0.0 {
                speed.push(1.0 / (row.get::<usize, f64>(1)? * 0.0006213712 * 60.0));
            } else {
                speed.push(0.0); // ideally this would just be a gap in the graph
            }
        }
        // these two may or may not have data available
        row.get::<usize, f64>(2)
            .into_iter()
            .for_each(|v| elevation.push(v * 3.28084));
        row.get::<usize, f64>(3)
            .into_iter()
            .for_each(|v| heart_rate.push(v));
    }

    let mut pace_plot = Plot::new(
        "".to_string(),
        "Distance [mi]".to_string(),
        "Pace [min/mile]".to_string(),
    );
    let series1_data: Vec<(f64, f64)> = distance
        .iter()
        .zip(speed.into_iter())
        .map(|(d, s)| (*d, s))
        .collect();
    pace_plot.add_series(DataSeries::new("Pace", &series1_data));

    let mut elev_plot = Plot::new(
        "".to_string(),
        "Distance [mi]".to_string(),
        "Elevation [ft]".to_string(),
    );
    let series2_data: Vec<(f64, f64)> = distance
        .iter()
        .zip(elevation.into_iter())
        .map(|(d, s)| (*d, s))
        .collect();
    elev_plot.show_y_zero = false;
    elev_plot.add_series(DataSeries::new("Elevation", &series2_data));

    let mut hr_plot = Plot::new(
        "".to_string(),
        "Distance [mi]".to_string(),
        "Heart Rate [bpm]".to_string(),
    );
    let series3_data: Vec<(f64, f64)> = distance
        .iter()
        .zip(heart_rate.into_iter())
        .map(|(d, s)| (*d, s))
        .collect();
    hr_plot.add_series(DataSeries::new("Heart Rate", &series3_data));

    // only plot if we have data
    let mut all_plots = Vec::with_capacity(3);
    if !series1_data.is_empty() {
        all_plots.push(&pace_plot);
    }
    if !series2_data.is_empty() {
        all_plots.push(&elev_plot);
    }
    if !series3_data.is_empty() {
        all_plots.push(&hr_plot);
    }
    plotter.plot(&all_plots)?;

    Ok(())
}
