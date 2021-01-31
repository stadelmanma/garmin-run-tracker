//! Service module that exports interfaces to external applications, APIs, etc.

pub mod elevation;
pub mod visualization;

// rexport some traits and utilty functions
pub use elevation::{new_elevation_handler, update_elevation_data, ElevationDataSource};
pub use visualization::plotting::{new_plotting_visualization_handler, DataPlottingService};
pub use visualization::route::{new_route_visualization_handler, RouteDrawingService};
