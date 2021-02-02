//! Import elevation data based on lat, long coordintes using the opentopodata API
use super::ElevationDataSource;
use crate::{
    config::{FromServiceConfig, ServiceConfig},
    gps::Location,
    Error,
};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{thread, time};

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

#[derive(Clone, Debug, FromServiceConfig)]
/// Defines the connection parameters to reqest elevation data from an instance of opentopodata
pub struct OpenTopoData {
    base_url: String,
    #[service_config(skip)]
    api_version: &'static str,
    dataset: String,
    batch_size: usize,
    requests_per_sec: f32,
}

impl OpenTopoData {
    /// Create a new data source that uses the OpenTopoData version 1 API
    pub fn new(
        base_url: String,
        dataset: String,
        batch_size: usize,
        requests_per_sec: f32,
    ) -> Self {
        OpenTopoData {
            base_url,
            api_version: "v1",
            dataset,
            batch_size,
            requests_per_sec,
        }
    }

    fn request_url(&self) -> String {
        format!("{}/{}/{}", self.base_url, self.api_version, self.dataset)
    }
}

impl Default for OpenTopoData {
    fn default() -> Self {
        OpenTopoData {
            base_url: "http://localhost:5000".to_string(),
            api_version: "v1",
            dataset: "ned10m".to_string(), // works well for USA/Canada
            batch_size: 100,
            requests_per_sec: -1.0,
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
        let delay = if self.requests_per_sec > 0.0 {
            (1.0e6 / self.requests_per_sec) as u64 // store as micro seconds
        } else {
            0 // treat zero as if a limit wasn't imposed to prevent subtle runtime error
        };
        let delay = time::Duration::from_micros(delay);

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
                return Err(Box::new(Error::RequestError(code, json.error)));
            }
            thread::sleep(delay);
        }

        Ok(())
    }
}
