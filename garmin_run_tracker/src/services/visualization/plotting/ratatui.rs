//! Use the ratatui crate to draw plots directly on the terminal
use super::{DataPlottingService, Plot};
use crate::config::{FromServiceConfig, ServiceConfig};
use crate::Error;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, GraphType},
    Terminal,
};
use std::cmp::max;
use std::io;

/// Defines parameters to interact with the MapBox API
#[derive(Debug, FromServiceConfig)]
pub struct TerminalPlotter {}

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

            for (&chunk, &plot) in chunks.into_iter().zip(plots) {
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
                let chart = Chart::new(datasets)
                    .block(Block::default().title(plot.title()))
                    .x_axis(
                        Axis::default()
                            .title(Span::styled(plot.x(), Style::default().fg(Color::Red)))
                            .style(Style::default().fg(Color::White))
                            .bounds([0.0, plot.xmax()])
                            .labels(plot.xticks()),
                    )
                    .y_axis(
                        Axis::default()
                            .title(Span::styled(plot.y(), Style::default().fg(Color::Red)))
                            .style(Style::default().fg(Color::White))
                            .bounds([plot.ymin(), plot.ymax()])
                            .labels(plot.yticks(y_nticks)),
                    );
                f.render_widget(chart, chunk);
            }
        })?;

        // we plot to the terminal so there isn't anything to return
        Ok(Vec::new())
    }
}
