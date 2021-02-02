//! Use an instance of open map tiles to draw a course route
use super::{Marker, RouteDrawingService};
use crate::config::{FromServiceConfig, ServiceConfig};
use crate::gps::{encode_coordinates, Location};
use crate::Error;
use log::warn;
use reqwest::blocking::Client;

/// Defines parameters to interact with the MapBox API
#[derive(Debug, FromServiceConfig)]
pub struct MapBox {
    base_url: String,
    api_version: String,
    username: String,
    style: String,
    image_width: u32,
    image_height: u32,
    marker_color: String,
    marker_style: String,
    stroke_color: String,
    stroke_width: u32,
    stroke_opacity: f32,
    access_token: String,
}

impl MapBox {
    fn request_url(&self, encoded_path: String, markers: &[Marker]) -> String {
        // hacky way to encode the path, we need to drop the leading '=' sign
        // from the call to form_urlencoded which is meant for key=value pairs
        let encoded_path = form_urlencoded::Serializer::new(String::new())
            .append_pair("", &encoded_path)
            .finish();
        let markers = markers.iter().fold(String::new(), |acc, m| {
            acc + &format!(
                "pin-{}-{}+{}({},{}),",
                self.marker_style,
                m.label().to_ascii_lowercase(),
                self.marker_color,
                m.longitude(),
                m.latitude()
            )
        });
        let markers = form_urlencoded::Serializer::new(String::new())
            .append_pair("", &markers)
            .finish();
        let url = format!(
            "{}/styles/{}/{}/{}/static/{}path-{}+{}-{}({})/auto/{}x{}",
            self.base_url,
            self.api_version,
            self.username,
            self.style,
            &markers[1..],
            self.stroke_width,
            self.stroke_color,
            self.stroke_opacity,
            &encoded_path[1..],
            self.image_width,
            self.image_height,
        );

        // mapbox has a URL limit of 8192 bytes, the access_token=[..] part in the query takes up
        // around 100 bytes by itself
        if url.len() > 8192 {
            warn!("URL length exceeds 8KB due to a long running route, request may fail (size={:.2}KB).", url.len() as f32/1024.0);
        }

        url
    }
}

impl Default for MapBox {
    fn default() -> Self {
        MapBox {
            base_url: "https://api.mapbox.com".to_string(),
            api_version: "v1".to_string(),
            username: "mapbox".to_string(),
            style: "streets-v11".to_string(),
            image_width: 1280,
            image_height: 1280,
            marker_color: "f07272".to_string(),
            marker_style: "l".to_string(),
            stroke_color: "f44".to_string(),
            stroke_width: 5,
            stroke_opacity: 0.75,
            access_token: String::new(),
        }
    }
}

impl RouteDrawingService for MapBox {
    fn draw_route(
        &self,
        trace: &[Location],
        markers: &[Marker],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // request image data
        let client = Client::new();
        let request_url = self.request_url(encode_coordinates(trace)?, markers);
        let resp = client
            .get(&request_url)
            .query(&[("access_token", &self.access_token)])
            .send()?;
        if resp.status().is_success() {
            // return image data
            match resp.bytes() {
                Ok(data) => Ok(data.into_iter().collect()),
                Err(e) => Err(Box::new(e)),
            }
        } else {
            let code = resp.status();
            Err(Box::new(Error::RequestError(
                code,
                "MapBox drawing failed".to_string(),
            )))
        }
    }
}
