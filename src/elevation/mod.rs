//! Access elevation data for a given GPS location using an external source
mod opentopodata;
pub use opentopodata::OpenTopoData;

/// Stores a single geospatial point
#[derive(Clone, Copy, Debug)]
pub struct Location {
    /// latitude coordinate in degrees
    latitude: f32,
    /// longitude coordinate in degrees
    longitude: f32,
    /// elevation in meters if available
    elevation: Option<f32>,
}

impl Location {
    /// Create a location without elevation data from coordinates provided in semicircles units
    pub fn from_fit_coordinates(latitude: i32, longitude: i32) -> Self {
        Location {
            latitude: (latitude as f32) * 180.0 / 2147483648.0,
            longitude: (longitude as f32) * 180.0 / 2147483648.0,
            elevation: None,
        }
    }

    /// Return latitude in degrees
    pub fn latitude(&self) -> f32 {
        self.latitude
    }

    /// Return longitude in degrees
    pub fn longitude(&self) -> f32 {
        self.longitude
    }

    /// Return elevation in meters (if defined)
    pub fn elevation(&self) -> Option<f32> {
        self.elevation
    }
}

/// trait that defines how elevation data should be added for an array of lat, long coordintes
pub trait ElevationDataSource {
    /// Updates the array of locations with elevation data
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>>;
}