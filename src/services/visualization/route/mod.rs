//! Plot running routes or course data using a GPS trace and a mapping source
use crate::config::ServiceConfig;
use crate::{Error, Location};
mod openmaptiles;
pub use openmaptiles::OpenMapTiles;

/// trait that defines how to process a vector of GPS traces into a route map
pub trait RouteDrawingService {
    /// Updates the array of locations with elevation data
    fn draw_route(&self, trace: &[Location]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

pub fn new_route_visualization_handler(
    config: &ServiceConfig,
) -> Result<Box<dyn RouteDrawingService>, Error> {
    match config.handler() {
        "openmaptiles" => Ok(Box::new(OpenMapTiles::from_config(config)?)),
        _ => Err(Error::UnknownServiceHandler(format!(
            "no route visualization handler exists for: {}",
            config.handler()
        ))),
    }
}
