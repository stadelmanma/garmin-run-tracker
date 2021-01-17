//! Use the tui crate to draw plots directly on the terminal
use super::{DataPlottingService, DataSeries, PlotLabels};
use crate::config::ServiceConfig;
use crate::Error;

/// Defines parameters to interact with the MapBox API
#[derive(Debug)]
pub struct TerminalPlotter {}

impl TerminalPlotter {
    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        Ok(base)
    }
}

impl Default for TerminalPlotter {
    fn default() -> Self {
        TerminalPlotter {}
    }
}

impl DataPlottingService for TerminalPlotter {
    fn plot(
        &self,
        series: &[DataSeries],
        x: &[f64],
        y: &[f64],
        labels: &PlotLabels,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}
