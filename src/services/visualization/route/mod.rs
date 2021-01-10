//! Plot running routes or course data using a GPS trace and a mapping source
use crate::config::ServiceConfig;
use crate::{Error, Location};
mod mapbox;
pub use mapbox::MapBox;
mod openmaptiles;
pub use openmaptiles::OpenMapTiles;

/// trait that defines how to process a vector of GPS traces into a route map
pub trait RouteDrawingService {
    /// Updates the array of locations with elevation data
    fn draw_route(
        &self,
        trace: &[Location],
        markers: &[Marker],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

/// Defines a marker at a specific GPS location that can be used by some route drawers to
/// annotate the map (e.g. adding mile markers)
pub struct Marker {
    location: Location,
    label: String,
}

impl Marker {
    pub fn new(location: Location, label: String) -> Self {
        Marker { location, label }
    }

    pub fn latitude(&self) -> f32 {
        self.location.latitude()
    }

    pub fn longitude(&self) -> f32 {
        self.location.longitude()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub fn new_route_visualization_handler(
    config: &ServiceConfig,
) -> Result<Box<dyn RouteDrawingService>, Error> {
    match config.handler() {
        "mapbox" => Ok(Box::new(MapBox::from_config(config)?)),
        "openmaptiles" => Ok(Box::new(OpenMapTiles::from_config(config)?)),
        _ => Err(Error::UnknownServiceHandler(format!(
            "no route visualization handler exists for: {}",
            config.handler()
        ))),
    }
}
