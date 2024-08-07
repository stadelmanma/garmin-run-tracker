//! Store application configuration that gets read from disk
use crate::services::{
    new_elevation_handler, new_plotting_visualization_handler, new_route_visualization_handler,
    DataPlottingService, ElevationDataSource, RouteDrawingService,
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
    DataPlotting,
    Elevation,
    RouteVisualization,
}

/// Type alias for clarity
pub type ServiceParameters = HashMap<String, Value>;

/// Configuration options for a single service of any type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    handler: String,
    configuration: ServiceParameters,
}

/// Trait that allows a specific kind of service to be created from it's config
pub trait FromServiceConfig: Default {
    fn from_config(config: &ServiceConfig) -> Result<Self, Error>;
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
                .ok_or_else(|| {
                    Error::InvalidConfigurationValue(format!(
                        "invalid value for {}.{}, expected a string: {:?}",
                        &self.handler, key, value
                    ))
                })
                .map(|v| v.to_string());
            Some(value)
        } else {
            None
        }
    }

    pub fn get_parameter_as_i64(&self, key: &str) -> Option<Result<i64, Error>> {
        if let Some(value) = self.configuration.get(key) {
            let value = value.as_i64().ok_or_else(|| {
                Error::InvalidConfigurationValue(format!(
                    "invalid value for {}.{}, expected an integer: {:?}",
                    &self.handler, key, value
                ))
            });
            Some(value)
        } else {
            None
        }
    }

    pub fn get_parameter_as_f64(&self, key: &str) -> Option<Result<f64, Error>> {
        if let Some(value) = self.configuration.get(key) {
            let value = value.as_f64().ok_or_else(|| {
                Error::InvalidConfigurationValue(format!(
                    "invalid value for {}.{}, expected a floating point value: {:?}",
                    &self.handler, key, value
                ))
            });
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
    epo_data_paths: Vec<String>,
    #[serde(
        deserialize_with = "deserialize_level_filter",
        serialize_with = "serialize_level_filter",
        default = "default_level_filter"
    )]
    log_level: LevelFilter,
    services: HashMap<ServiceType, ServiceConfig>,
}

impl Config {
    pub fn load<T: Read>(source: &mut T) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(source)
    }

    pub fn import_paths(&self) -> &[String] {
        &self.import_paths
    }

    pub fn epo_data_paths(&self) -> &[String] {
        &self.epo_data_paths
    }

    pub fn log_level(&self) -> LevelFilter {
        self.log_level
    }

    pub fn get_elevation_handler(&self) -> Result<Box<dyn ElevationDataSource>, Error> {
        match self.services.get(&ServiceType::Elevation) {
            Some(cfg) => new_elevation_handler(cfg),
            None => Err(Error::UnknownServiceHandler(
                "no service configuration defined for elevation".to_string(),
            )),
        }
    }

    pub fn get_route_visualization_handler(&self) -> Result<Box<dyn RouteDrawingService>, Error> {
        match self.services.get(&ServiceType::RouteVisualization) {
            Some(cfg) => new_route_visualization_handler(cfg),
            None => Err(Error::UnknownServiceHandler(
                "no service configuration defined for route visualization".to_string(),
            )),
        }
    }

    pub fn get_plotting_visualization_handler(
        &self,
    ) -> Result<Box<dyn DataPlottingService>, Error> {
        match self.services.get(&ServiceType::DataPlotting) {
            Some(cfg) => new_plotting_visualization_handler(cfg),
            None => {
                // use terminal as default plotter since we always have that
                new_plotting_visualization_handler(&ServiceConfig {
                    handler: "ratatui".to_string(),
                    configuration: HashMap::new(),
                })
            }
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

fn default_level_filter() -> LevelFilter {
    LevelFilter::Info
}
