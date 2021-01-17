//! Plot running data for a given FIT file using a plotting backend
use crate::config::ServiceConfig;
use crate::Error;
mod tui;
pub use self::tui::TerminalPlotter;

/// alias for a vector of (x, y) coordinate pairs
pub type DataSeries<'a> = &'a [(f64, f64)];

/// Defines the labels applied to the plot
#[derive(Debug)]
pub struct PlotLabels {
    series: Vec<String>,
    x_axis: String,
    y_axis: String,
    title: String,
}

/// trait that defines how to plot a set of data series
pub trait DataPlottingService {
    /// Draw a plot of data to display to the user
    fn plot(
        &self,
        series: &[DataSeries],
        x: &[f64],
        y: &[f64],
        labels: &PlotLabels,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

pub fn new_plotting_visualization_handler(
    config: &ServiceConfig,
) -> Result<Box<dyn DataPlottingService>, Error> {
    match config.handler() {
        "tui" => Ok(Box::new(TerminalPlotter::from_config(config)?)),
        _ => Err(Error::UnknownServiceHandler(format!(
            "no plotting visualization handler exists for: {}",
            config.handler()
        ))),
    }
}
