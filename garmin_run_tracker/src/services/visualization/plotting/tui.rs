//! Use the tui crate to draw plots directly on the terminal
use super::{DataPlottingService, Plot};
use crate::config::{FromServiceConfig, ServiceConfig};
use crate::Error;
use std::cmp::max;
use std::io;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, GraphType},
    Terminal,
};

/// Defines parameters to interact with the MapBox API
#[derive(Debug)]
pub struct TerminalPlotter {}

impl FromServiceConfig for TerminalPlotter {
    fn from_config(_config: &ServiceConfig) -> Result<Self, Error> {
        Ok(Self::default())
    }
}

impl Default for TerminalPlotter {
    fn default() -> Self {
        TerminalPlotter {}
    }
}

impl DataPlottingService for TerminalPlotter {
    fn plot(&self, plots: &[&Plot]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;
        terminal.draw(|f| {
            let constraints = vec![Constraint::Ratio(1, plots.len() as u32); plots.len()];
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(constraints)
                .split(f.size());
            let y_nticks = max(2, 7 - plots.len()); // reduce ticks if less vertical space
            for (chunk, plot) in chunks.into_iter().zip(plots) {
                let datasets = plot
                    .series()
                    .iter()
                    .map(|s| {
                        Dataset::default()
                            //.name(s.name())
                            .marker(symbols::Marker::Braille)
                            .graph_type(GraphType::Line)
                            .style(Style::default().fg(Color::Cyan))
                            .data(s.data())
                    })
                    .collect();
                // fetch min and maximum values for axes across all data series
                let mut x_min = 0f64;
                let mut x_max = 1f64;
                let mut y_min = 0f64;
                let mut y_max = 1f64;
                for series in plot.series() {
                    for (x, y) in series {
                        if x < x_min {
                            x_min = x;
                        }
                        if x > x_max {
                            x_max = x;
                        }
                        if y < y_min {
                            y_min = y;
                        }
                        if y > y_max {
                            y_max = y;
                        }
                    }
                }
                y_max *= 1.1;
                let chart = Chart::new(datasets)
                    .block(Block::default().title(plot.title()))
                    .x_axis(
                        Axis::default()
                            .title(Span::styled(plot.x(), Style::default().fg(Color::Red)))
                            .style(Style::default().fg(Color::White))
                            .bounds([x_min, x_max])
                            .labels(
                                (0..=5)
                                    .map(|n| Span::from(format!("{:.3}", x_max * (n as f64 / 5.0))))
                                    .collect(),
                            ),
                    )
                    .y_axis(
                        Axis::default()
                            .title(Span::styled(plot.y(), Style::default().fg(Color::Red)))
                            .style(Style::default().fg(Color::White))
                            .bounds([y_min, y_max])
                            .labels(
                                (0..=y_nticks)
                                    .map(|n| Span::from(format!("{:.3}", y_max * (n as f64 / 5.0))))
                                    .collect(),
                            ),
                    );
                f.render_widget(chart, chunk);
            }
        })?;

        // we plot to the terminal so there isn't anything to return
        Ok(Vec::new())
    }
}
