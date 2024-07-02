//! Plot running data for a given FIT file using a plotting backend
use ::ratatui::text::Span;

use crate::config::{FromServiceConfig, ServiceConfig};
use crate::Error;
mod ratatui;
pub use self::ratatui::TerminalPlotter;

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

impl<'a> IntoIterator for &DataSeries<'a> {
    type Item = (f64, f64);
    type IntoIter = DataSeriesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DataSeriesIter {
            data: self.data(),
            idx: 0,
        }
    }
}

pub struct DataSeriesIter<'a> {
    data: &'a [(f64, f64)],
    idx: usize,
}

impl<'a> Iterator for DataSeriesIter<'a> {
    // we will be counting with usize
    type Item = (f64, f64);

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        // Check to see if we've finished counting or not.
        self.idx += 1;
        match self.data.get(self.idx) {
            Some(v) => {
                //self.idx += 1;
                Some(*v)
            }
            None => {
                //self.idx = 0; // reset counter so we can loop over this again
                None
            }
        }
    }
}

/// Defines the labels applied to the plot
#[derive(Debug)]
pub struct Plot<'a> {
    title: String,
    x_axis: String,
    y_axis: String,
    /// Ensure 0 is shown on plot x axis, default true
    pub show_x_zero: bool,
    /// Ensure 0 is shown on plot y axis, default true
    pub show_y_zero: bool,
    series: Vec<DataSeries<'a>>,
    _xmax: f64,
    _ymin: f64,
    _ymax: f64,
}

impl<'a> Plot<'a> {
    pub fn new(title: String, x_axis: String, y_axis: String) -> Self {
        Plot {
            series: Vec::new(),
            show_x_zero: true,
            show_y_zero: true,
            _xmax: 0.0,
            _ymin: 1e99f64,
            _ymax: 0.0,
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
        for (x, y) in &data {
            if x > self._xmax {
                self._xmax = x;
            }
            if y < self._ymin {
                self._ymin = y;
            }
            if y > self._ymax {
                self._ymax = y;
            }
        }
        self.series.push(data);
    }

    pub fn xmax(&self) -> f64 {
        self._xmax
    }

    pub fn ymin(&self) -> f64 {
        if self.show_y_zero {
            0.0
        } else {
            self._ymin
        }
    }

    pub fn ymax(&self) -> f64 {
        // bump by 10% of range
        self._ymax + 0.10 * (self._ymax - self._ymin)
    }

    pub fn xticks(&self) -> Vec<Span> {
        let mut ticks: Vec<Span> = (0..=(self._xmax.floor() as i32))
            .map(|v| Span::from(v.to_string()))
            .collect();
        ticks.push(Span::from(format!("{:0.1}", self._xmax)));
        return ticks;
    }

    pub fn yticks(&self, nticks: usize) -> Vec<Span> {
        (0..=nticks)
            .map(|n| {
                Span::from(format!(
                    "{:.3}",
                    self.ymin() + (self.ymax() - self.ymin()) * (n as f64 / nticks as f64)
                ))
            })
            .collect()
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
        "ratatui" => Ok(Box::new(TerminalPlotter::from_config(config)?)),
        _ => Err(Error::UnknownServiceHandler(format!(
            "no plotting visualization handler exists for: {}",
            config.handler()
        ))),
    }
}
