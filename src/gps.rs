//! Module with GPS specific structures
use std::char;

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

    /// Return elevation in meters (if defined)
    pub fn set_elevation(&mut self, elevation: Option<f32>) {
        self.elevation = elevation;
    }
}

/// Encodes a slice of coordinates into Google Encoded Polyline format.
///
/// This code was extracted and simplified for our use case from:
/// https://github.com/georust/polyline
/// https://developers.google.com/maps/documentation/utilities/polylinealgorithm
pub fn encode_coordinates(coordinates: &[Location]) -> Result<String, String> {
    let mut output = "".to_string();
    let mut b = (0, 0);

    for a in coordinates {
        let a = (scale(a.latitude), scale(a.longitude));
        output = output + &encode(a.0, b.0)?;
        output = output + &encode(a.1, b.1)?;
        b = a;
    }

    Ok(output)
}

/// Scale a floating point value into an integer at the given precision
#[inline]
fn scale(n: f32) -> i32 {
    static FACTOR: f32 = 100_000.0; // use 5 digits of precision
    (FACTOR * n).round() as i32
}

/// Encode a single latitude or longitude value into the polyline format
fn encode(current: i32, previous: i32) -> Result<String, String> {
    let mut coordinate = (current - previous) << 1;
    if (current - previous) < 0 {
        coordinate = !coordinate;
    }
    let mut output: String = "".to_string();
    while coordinate >= 0x20 {
        let from_char = char::from_u32(((0x20 | (coordinate & 0x1f)) + 63) as u32)
            .ok_or("Couldn't convert character")?;
        output.push(from_char);
        coordinate >>= 5;
    }
    let from_char = char::from_u32((coordinate + 63) as u32).ok_or("Couldn't convert character")?;
    output.push(from_char);
    Ok(output)
}
