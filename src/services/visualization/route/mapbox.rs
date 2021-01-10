//! Use an instance of open map tiles to draw a course route
use super::{Marker, RouteDrawingService};
use crate::config::ServiceConfig;
use crate::{encode_coordinates, Error, Location};
use form_urlencoded;
use log::warn;
use reqwest::blocking::Client;
use std::iter::FromIterator;

/// Defines parameters to interact with the MapBox API
#[derive(Debug)]
pub struct MapBox {
    base_url: String,
    api_version: String,
    username: String,
    style: String,
    image_width: u32,
    image_height: u32,
    stroke_color: String,
    stroke_width: u32,
    stroke_opacity: f32,
    access_token: Option<String>,
}

impl MapBox {
    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        for key in config.parameters() {
            match key.as_ref() {
                "access_token" => {
                    base.access_token = config.get_parameter_as_string(key).transpose()?
                }
                _ => warn!(
                    "unknown configuration parameter for MapBox: {}={:?}",
                    key,
                    config.get_parameter(key)
                ),
            }
        }
        Ok(base)
    }

    fn request_url(&self, encoded_path: String) -> String {
        // hacky way to encode the path, we need to drop the leading '=' sign
        // from the call to form_urlencoded which is meant for key=value pairs
        let encoded_path: String = form_urlencoded::Serializer::new(String::new())
            .append_pair("", &encoded_path)
            .finish();
        let url = format!(
            "{}/styles/{}/{}/{}/static/path-{}+{}-{}({})/auto/{}x{}",
            self.base_url,
            self.api_version,
            self.username,
            self.style,
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
            stroke_color: "f44".to_string(),
            stroke_width: 3,
            stroke_opacity: 0.50,
            access_token: None,
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
        let request_url = self.request_url(encode_coordinates(trace)?);
        let resp = client
            .get(&request_url)
            .query(&[("access_token", self.access_token.as_ref())])
            .send()?;
        if resp.status().is_success() {
            // return image data
            return match resp.bytes() {
                Ok(data) => Ok(Vec::from_iter(data.into_iter())),
                Err(e) => Err(Box::new(e)),
            };
        } else {
            let code = resp.status();
            return Err(Box::new(Error::Other(format!(
                "MapBox drawing failed with code: {}",
                code
            ))));
        }
    }
}