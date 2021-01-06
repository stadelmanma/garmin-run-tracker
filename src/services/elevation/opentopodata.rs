//! Import elevation data based on lat, long coordintes using the opentopodata API
use super::ElevationDataSource;
use crate::config::ServiceConfig;
use crate::{Error, Location};
use log::warn;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
struct Elevation {
    elevation: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct SuccessResponse {
    results: Vec<Elevation>,
}

#[derive(Clone, Debug)]
/// Defines the connection parameters to reqest elevation data from an instance of opentopodata
pub struct OpenTopoData {
    base_url: String,
    dataset: String,
    batch_size: usize,
}

impl OpenTopoData {
    /// Create a new data source that uses the OpenTopoData version 1 API
    pub fn new(base_url: String, dataset: String, batch_size: usize) -> Self {
        OpenTopoData {
            base_url,
            dataset,
            batch_size,
        }
    }

    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        for key in config.parameters() {
            match key.as_ref() {
                "base_url" => {
                    if let Some(val) = config.get_parameter_as_string(key) {
                        base.base_url = val?
                    };
                }
                "dataset" => {
                    if let Some(val) = config.get_parameter_as_string(key) {
                        base.dataset = val?
                    };
                }
                "batch_size" => {
                    if let Some(val) = config.get_parameter_as_i64(key) {
                        base.batch_size = val? as usize
                    };
                }
                _ => warn!(
                    "unknown configuration parameter for OpenTopoData: {}={:?}",
                    key,
                    config.get_parameter(key)
                ),
            }
        }

        Ok(base)
    }

    fn request_url(&self) -> String {
        format!("{}/v1/{}", self.base_url, self.dataset)
    }
}

impl Default for OpenTopoData {
    fn default() -> Self {
        OpenTopoData {
            base_url: "http://localhost:5000".to_string(),
            dataset: "ned10m".to_string(), // works well for USA/Canada
            batch_size: 100,
        }
    }
}

impl ElevationDataSource for OpenTopoData {
    fn request_elevation_data(
        &self,
        locations: &mut [Location],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // define base url and batch size as setup in opentopodata instance
        let request_url = self.request_url();

        // create client and start fetching data in batches
        let client = Client::new();
        for chunk in locations.chunks_mut(self.batch_size) {
            let loc_params: String = chunk
                .iter()
                .map(|l| format!("{0:.6},{1:.6}", l.latitude(), l.longitude()))
                .collect::<Vec<String>>()
                .join("|");
            let resp = client
                .get(&request_url)
                .query(&[("locations", &loc_params)])
                .send()?;
            if resp.status().is_success() {
                // parse response and update locations
                let json: SuccessResponse = resp.json()?;
                for (loc, elevation) in chunk
                    .iter_mut()
                    .zip(json.results.into_iter().map(|r| r.elevation))
                {
                    loc.set_elevation(elevation);
                }
            } else {
                // parse error response to get reason why the request failed
                let code = resp.status();
                let json: ErrorResponse = resp.json()?;
                return Err(Box::new(Error::ElevationRequestError(code, json.error)));
            }
        }

        Ok(())
    }
}
