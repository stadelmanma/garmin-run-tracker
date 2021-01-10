//! Use an instance of open map tiles to draw a course route
use super::RouteDrawingService;
use crate::config::ServiceConfig;
use crate::{encode_coordinates, Error, Location};
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
        format!(
            "{}/styles/{}/{}/{}/static/path-{}+{}-{}({})/auto/{}x{}",
            self.base_url,
            self.api_version,
            self.username,
            self.style,
            self.stroke_width,
            self.stroke_color,
            self.stroke_opacity,
            encoded_path,
            self.image_width,
            self.image_height,
        )
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
    fn draw_route(&self, trace: &[Location]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
