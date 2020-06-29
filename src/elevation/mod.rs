//! Access elevation data for a given GPS location using an external source
use crate::Location;
mod opentopodata;
pub use opentopodata::OpenTopoData;

/// trait that defines how elevation data should be added for an array of lat, long coordintes
pub trait ElevationDataSource {
    /// Updates the array of locations with elevation data
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>>;
}
