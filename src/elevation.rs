//! Access elevation data for a given GPS location using an external source
use crate::Error;
use reqwest::blocking::Client;
use serde::Deserialize;


/// Stores a single geospatial point
#[derive(Clone, Copy, Debug)]
pub struct Location {
    /// latitude coordinate in degrees
    latitude: f32,
    /// longitude coordinate in degrees
    longitude: f32,
    /// elevation in meters if available
    elevation: Option<f32>
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

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
struct Elevation {
    elevation: Option<f32>
}

#[derive(Debug, Deserialize)]
struct SuccessResponse {
    results: Vec<Elevation>,
}

pub fn request_elevation_data(locations: &mut [Location]) -> Result<(), Box<dyn std::error::Error>> {
    // define base url and batch size as setup in opentopodata instance
    let batch_size = 100;
    let base_url = "http://localhost:5000/v1/ned10m";

    // create client and start fetching data in batches
    let client = Client::new();
    for chunk in locations.chunks_mut(batch_size) {
        let loc_params: String = chunk.iter().map(|l| format!("{0:.6},{1:.6}", l.latitude, l.longitude)).collect::<Vec<String>>().join("|");
        let resp = client.get(base_url)
                            .query(&[("locations", &loc_params)])
                            .send()?;
        if resp.status().is_success() {
            // parse response and update locations
            let json: SuccessResponse = resp.json()?;
            for (loc, elevation) in chunk.iter_mut().zip(json.results.into_iter().map(|r| r.elevation)) {
                loc.elevation = elevation;
            }
        }
        else {
            // parse error response to get reason why the request failed
            let code = resp.status();
            let json: ErrorResponse = resp.json()?;
            return Err(Box::new(Error::ElevationRequestError(code, json.error)));
        }
    }

    Ok(())
}
