//! Store application configuration that gets read from disk
use crate::services::{
    new_elevation_handler, new_route_visualization_handler, ElevationDataSource,
    RouteDrawingService,
};
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_yaml::Value;
use simplelog::LevelFilter;
use std::collections::HashMap;
use std::io::prelude::*;
use std::iter::Iterator;
use std::str::FromStr;

/// Defines the allowed keys under the services map
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Elevation,
    RouteVisualization,
}

pub type ServiceParameters = HashMap<String, Value>;

/// Type alias for clarity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    handler: String,
    configuration: ServiceParameters,
}

impl ServiceConfig {
    pub fn handler(&self) -> &str {
        &self.handler
    }

    pub fn parameters(&self) -> impl Iterator<Item = &String> + '_ {
        self.configuration.keys()
    }

    pub fn get_parameter(&self, key: &str) -> Option<&Value> {
        self.configuration.get(key)
    }

    pub fn get_parameter_as_string(&self, key: &str) -> Option<Result<String, Error>> {
        if let Some(value) = self.configuration.get(key) {
            let value = value
                .as_str()
                .ok_or(Error::InvalidConfigurationValue(format!(
                    "invalid value for {}.{}, expected a string: {:?}",
                    &self.handler, key, value
                )))
                .map(|v| v.to_string());
            Some(value)
        } else {
            None
        }
    }

    pub fn get_parameter_as_i64(&self, key: &str) -> Option<Result<i64, Error>> {
        if let Some(value) = self.configuration.get(key) {
            let value = value
                .as_i64()
                .ok_or(Error::InvalidConfigurationValue(format!(
                    "invalid value for {}.{}, expected an integer: {:?}",
                    &self.handler, key, value
                )));
            Some(value)
        } else {
            None
        }
    }
}

/// Configuration struct that we can create from the config file used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    import_paths: Vec<String>,
    #[serde(
        deserialize_with = "deserialize_level_filter",
        serialize_with = "serialize_level_filter"
    )]
    log_level: LevelFilter,
    services: HashMap<ServiceType, ServiceConfig>,
}

impl Config {
    pub fn load<T: Read>(source: &mut T) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(source)
    }

    pub fn get_elevation_handler(&self) -> Result<impl ElevationDataSource, Error> {
        match self.services.get(&ServiceType::Elevation) {
            Some(cfg) => new_elevation_handler(cfg),
            None => Err(Error::UnknownServiceHandler(
                "no service configuration defined for elevation".to_string(),
            )),
        }
    }

    pub fn get_route_visualization_handler(&self) -> Result<impl RouteDrawingService, Error> {
        match self.services.get(&ServiceType::RouteVisualization) {
            Some(cfg) => new_route_visualization_handler(cfg),
            None => Err(Error::UnknownServiceHandler(
                "no service configuration defined for elevation".to_string(),
            )),
        }
    }
}

fn deserialize_level_filter<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    LevelFilter::from_str(&buf)
        .map_err(|_| serde::de::Error::custom(format!("invalid level value: {}", buf)))
}

fn serialize_level_filter<S>(level: &LevelFilter, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&level.to_string())
}