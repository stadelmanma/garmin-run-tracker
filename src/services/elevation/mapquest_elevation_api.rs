//! Import elevation data based on lat, long coordintes using the mapquest open elevation API
use super::ElevationDataSource;
use crate::config::ServiceConfig;
use crate::gps::Location;
use crate::{
    set_float_param_from_config, set_int_param_from_config, set_string_param_from_config, Error,
};
use log::warn;
use reqwest::{blocking::Client, Url};
use serde::Deserialize;

#[derive(Clone, Debug)]
/// Defines the connection parameters to reqest elevation data from an instance of opentopodata
pub struct MapquestElevationApi {
    base_url: &'static str,
    api_version: &'static str,
    api_key: String,
    batch_size: u32,
}

impl MapquestElevationApi {
    /// Create a new data source that uses the OpenTopoData version 1 API
    pub fn new(api_key: String) -> Self {
        let base = Self::default();
        base.api_key = api_key;
        base
    }

    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        for key in config.parameters() {
            match key.as_ref() {
                "api_key" => set_string_param_from_config!(base, api_key, config),
                "batch_size" => set_int_param_from_config!(base, batch_size, config, u32),
                _ => warn!(
                    "unknown configuration parameter for MapquestElevationApi: {}={:?}",
                    key,
                    config.get_parameter(key)
                ),
            }
        }
        Ok(base)
    }

    fn request_url(&self, encoded_path: String) -> Result<Url, Box<dyn std::error::Error>> {
        // hacky way to encode the path, we need to drop the leading '=' sign
        // from the call to form_urlencoded which is meant for key=value pairs
        Url::parse_with_params(
            &format!("{}/elevation/{}/profile?", self.base_url, self.api_version),
            &[
                ("key", self.api_key),
                ("shapeFormat", "cmp".to_string()),
                ("latLngCollection", encoded_path),
            ],
        )
        .map_err(|e| e.into())
    }
}

impl Default for MapquestElevationApi {
    fn default() -> Self {
        MapquestElevationApi {
            base_url: "http://open.mapquestapi.com",
            api_version: "v1",
            api_key: String::new(),
            batch_size: 512,
        }
    }
}

impl ElevationDataSource for MapquestElevationApi {
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // invalid value: -32768

        Ok(())
    }
}
