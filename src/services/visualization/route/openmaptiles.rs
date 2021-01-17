//! Use an instance of open map tiles to draw a course route
use super::{Marker, RouteDrawingService};
use crate::config::ServiceConfig;
use crate::gps::Location;
use crate::{set_int_param_from_config, set_string_param_from_config, Error};
use log::warn;
use reqwest::blocking::Client;
use std::iter::FromIterator;

/// Defines connection parameters to request course rotes from an OpenMapTiles server
#[derive(Debug)]
pub struct OpenMapTiles {
    base_url: String,
    style: String,
    image_width: u32,
    image_height: u32,
    image_format: String,
    stroke_color: String,
    stroke_width: u32,
}

impl OpenMapTiles {
    pub fn new(base_url: String, style: String) -> Self {
        let mut omt: OpenMapTiles = Default::default();
        omt.base_url = base_url;
        omt.style = style;
        omt
    }

    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        for key in config.parameters() {
            match key.as_ref() {
                "base_url" => set_string_param_from_config!(base, base_url, config),
                "style" => set_string_param_from_config!(base, style, config),
                "image_width" => set_int_param_from_config!(base, image_width, config, u32),
                "image_height" => set_int_param_from_config!(base, image_height, config, u32),
                "image_format" => set_string_param_from_config!(base, image_format, config),
                "stroke_color" => set_string_param_from_config!(base, stroke_color, config),
                "stroke_width" => set_int_param_from_config!(base, stroke_width, config, u32),
                _ => warn!(
                    "unknown configuration parameter for OpenMapTiles: {}={:?}",
                    key,
                    config.get_parameter(key)
                ),
            }
        }

        Ok(base)
    }

    pub fn image_width(&self) -> u32 {
        self.image_width
    }

    pub fn set_image_width(&mut self, width: u32) {
        self.image_width = width;
    }

    pub fn image_height(&self) -> u32 {
        self.image_height
    }

    pub fn set_image_height(&mut self, height: u32) {
        self.image_height = height;
    }

    pub fn stroke_color(&self) -> &str {
        &self.stroke_color
    }

    pub fn set_stroke_color(&mut self, color: String) {
        self.stroke_color = color;
    }

    pub fn stroke_width(&self) -> u32 {
        self.stroke_width
    }

    pub fn set_stroke_width(&mut self, width: u32) {
        self.stroke_width = width;
    }

    fn request_url(&self, min_lat: f32, max_lat: f32, min_lon: f32, max_lon: f32) -> String {
        // Ex.: http://localhost:8080/styles/osm-bright/static/-80.1465,39.46,-80.1313,39.4842/1800x1200.png
        format!(
            "{}/styles/{}/static/{},{},{},{}/{}x{}.{}",
            self.base_url,
            self.style,
            min_lon,
            min_lat,
            max_lon,
            max_lat,
            self.image_width,
            self.image_height,
            self.image_format
        )
    }
}

impl Default for OpenMapTiles {
    fn default() -> Self {
        OpenMapTiles {
            base_url: "http://localhost:8080".to_string(),
            style: "osm-bright".to_string(),
            image_width: 1800,
            image_height: 1200,
            image_format: "png".to_string(), // other formats are available but the list is short,
            stroke_color: "red".to_string(),
            stroke_width: 3,
        }
    }
}

impl RouteDrawingService for OpenMapTiles {
    fn draw_route(
        &self,
        trace: &[Location],
        _markers: &[Marker],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // build path query while determining the bounding coordintes
        let mut min_lat = 90.0;
        let mut max_lat = -90.0;
        let mut min_lon = 180.0;
        let mut max_lon = -180.0;
        let mut path = String::new();
        for location in trace {
            if location.latitude() < min_lat {
                min_lat = location.latitude()
            } else if location.latitude() > max_lat {
                max_lat = location.latitude()
            }

            if location.longitude() < min_lon {
                min_lon = location.longitude()
            } else if location.longitude() > max_lon {
                max_lon = location.longitude()
            }
            path += &format!("{},{}|", location.longitude(), location.latitude());
        }
        path.truncate(path.len() - 1); // remove trailing pipe

        // request image data
        let client = Client::new();
        let request_url = self.request_url(min_lat, max_lat, min_lon, max_lon);
        let resp = client
            .get(&request_url)
            .query(&[("stroke", self.stroke_color())])
            .query(&[("width", self.stroke_width())])
            .query(&[("path", &path)])
            .send()?;
        if resp.status().is_success() {
            // return image data
            return match resp.bytes() {
                Ok(data) => Ok(Vec::from_iter(data.into_iter())),
                Err(e) => Err(Box::new(e)),
            };
        } else {
            let code = resp.status();
            return Err(Box::new(Error::RequestError(
                code,
                "OpenMapTiles drawing failed".to_string(),
            )));
        }
    }
}
