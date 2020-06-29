//! Import elevation data based on lat, long coordintes using the opentopodata API
use super::ElevationDataSource;
use crate::{Error, Location};
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

    fn request_url(&self) -> String {
        format!("{}/v1/{}", self.base_url, self.dataset)
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
