//! Service module that exports interfaces to external applications, APIs, etc.

pub mod elevation;
pub mod visualization;

// rexport some traits and utilty functions
pub use elevation::{update_elevation_data, ElevationDataSource};
pub use visualization::route::RouteDrawingService;
