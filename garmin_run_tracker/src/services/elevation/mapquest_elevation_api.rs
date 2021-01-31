//! Import elevation data based on lat, long coordintes using the mapquest open elevation API
use super::ElevationDataSource;
use crate::{
    config::ServiceConfig,
    gps::{encode_coordinates, Location},
    set_int_param_from_config, set_string_param_from_config, Error,
};
use log::warn;
use reqwest::{blocking::Client, StatusCode, Url};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Elevation {
    distance: f32,
    #[serde(deserialize_with = "deserialize_heightr")]
    height: Option<f32>, // invalid value: -32768, so I need to check for it
}

fn deserialize_heightr<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = f32::deserialize(deserializer)?;
    if (value as i32) == -32768 {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

#[derive(Debug, Deserialize)]
struct Info {
    copyright: HashMap<String, String>,
    statuscode: u16,
    messages: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Response {
    shape_points: Vec<f32>,
    elevation_profile: Vec<Elevation>,
    info: Info,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            shape_points: Vec::new(),
            elevation_profile: Vec::new(),
            info: Info {
                copyright: HashMap::new(),
                statuscode: 0,
                messages: Vec::new(),
            },
        }
    }
}

#[derive(Clone, Debug)]
/// Defines the connection parameters to reqest elevation data from an instance of opentopodata
pub struct MapquestElevationApi {
    base_url: &'static str,
    api_version: &'static str,
    api_key: String,
    batch_size: usize,
}

impl MapquestElevationApi {
    /// Create a new data source that uses the OpenTopoData version 1 API
    pub fn new(api_key: String) -> Self {
        MapquestElevationApi {
            api_key,
            ..Default::default()
        }
    }

    pub fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
        let mut base = Self::default();
        for key in config.parameters() {
            match key.as_ref() {
                "api_key" => set_string_param_from_config!(base, api_key, config),
                "batch_size" => set_int_param_from_config!(base, batch_size, config, usize),
                _ => warn!(
                    "unknown configuration parameter for MapquestElevationApi: {}={:?}",
                    key,
                    config.get_parameter(key)
                ),
            }
        }
        Ok(base)
    }

    fn request_url(&self) -> Result<Url, Box<dyn std::error::Error>> {
        Url::parse_with_params(
            &format!("{}/elevation/{}/profile?", self.base_url, self.api_version),
            &[("key", self.api_key()), ("shapeFormat", "cmp")],
        )
        .map_err(|e| e.into())
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
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
        // create client and start fetching data in batches
        let client = Client::new();
        for chunk in locations.chunks_mut(self.batch_size) {
            let request_url = self.request_url()?;
            let resp = client
                .get(request_url)
                .query(&[("latLngCollection", &encode_coordinates(chunk)?)])
                .send()?;
            if resp.status().is_success() {
                // parse response and update locations, they seem to use 0 as a success response code
                // but lets check for 200 as well since that is standard
                let json: Response = resp.json()?;
                if json.info.statuscode == 0 || json.info.statuscode == 200 {
                    for (loc, elevation) in chunk
                        .iter_mut()
                        .zip(json.elevation_profile.into_iter().map(|r| r.height))
                    {
                        loc.set_elevation(elevation);
                    }
                } else {
                    return Err(Box::new(Error::RequestError(
                        StatusCode::from_u16(json.info.statuscode)?,
                        json.info.messages.join("\n"),
                    )));
                }
            } else {
                // parse error response to get reason why the request failed
                let code = resp.status();
                return Err(Box::new(Error::RequestError(code, String::new())));
            }
        }

        Ok(())
    }
}
