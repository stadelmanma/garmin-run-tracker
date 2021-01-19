//! Plot running data for a given FIT file using a plotting backend
use crate::config::ServiceConfig;
use crate::Error;
mod tui;
pub use self::tui::TerminalPlotter;

/// A vector of (x, y) coordinate pairs and a name
#[derive(Debug)]
pub struct DataSeries<'a> {
    name: &'a str,
    data: &'a [(f64, f64)],
}

impl<'a> DataSeries<'a> {
    pub fn new(name: &'a str, data: &'a [(f64, f64)]) -> Self {
        DataSeries { name, data }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn data(&self) -> &'a [(f64, f64)] {
        self.data
    }
}

/// Defines the labels applied to the plot
#[derive(Debug)]
pub struct Plot<'a> {
    title: String,
    x_axis: String,
    y_axis: String,
    series: Vec<DataSeries<'a>>,
}

impl<'a> Plot<'a> {
    pub fn new(title: String, x_axis: String, y_axis: String) -> Self {
        Plot {
            series: Vec::new(),
            x_axis,
            y_axis,
            title,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn x(&self) -> &str {
        &self.x_axis
    }

    pub fn y(&self) -> &str {
        &self.y_axis
    }

    pub fn series(&self) -> &[DataSeries<'a>] {
        &self.series
    }

    pub fn add_series(&mut self, data: DataSeries<'a>) {
        self.series.push(data);
    }
}

/// trait that defines how to plot a set of data series
pub trait DataPlottingService {
    /// Draw a plot of data to display to the user
    fn plot(&self, plots: &[&Plot]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
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
