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
    title: String,
    x_axis: String,
    y_axis: String,
    series: Vec<String>,
}

impl PlotLabels {
    pub fn new(title: String, x_axis: String, y_axis: String) -> Self {
        PlotLabels {
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

    pub fn series(&self) -> &[String] {
        &self.series
    }

    pub fn add_series_label(&mut self, label: String) {
        self.series.push(label);
    }
}

/// trait that defines how to plot a set of data series
pub trait DataPlottingService {
    /// Draw a plot of data to display to the user
    fn plot(
        &self,
        series: &[DataSeries],
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
