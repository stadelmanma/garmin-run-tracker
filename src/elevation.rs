//! Access elevation data for a given GPS location using an external source
use reqwest::blocking::Client;

// url format
// http://localhost:5000/v1/ned10m?locations=39.123456,-80.123456|lat,long

// maybe use polyline eventually to send more points
// https://github.com/georust/polyline


/// Stores a single geospatial point
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
        // todo: parse response
        println!("{:#?}", resp);

        for loc in chunk.iter_mut() {
            // todo add elevation if it exists to the location in the chunk
        }
    }

    Ok(())
}
