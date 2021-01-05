//! Store application configuration that gets read from disk
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_yaml::Value;
use simplelog::LevelFilter;
use std::collections::HashMap;
use std::io::prelude::*;
use std::str::FromStr;

/// Defines the allowed keys under the services map
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Elevation,
    VisualizationRoute,
}

/// Type alias for clarity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    handler: String,
    configuration: HashMap<String, Value>,
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
