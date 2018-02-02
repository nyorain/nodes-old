use super::toml;
use super::storage;

use std::io;
use std::fs;
use std::env;

use std::io::prelude::*;
use std::path::PathBuf;
use std::fs::File;
use std::collections::HashSet;
use std::collections::HashMap;
use toml::ValueImpl;

const WRITE_INITIAL_CONFIG: bool = true;

pub struct StorageConfig {
    default: String,
    storages: HashMap<String, PathBuf>
}

pub struct Config {
    value: toml::Value,
    storage: StorageConfig
}

#[derive(Deserialize, Serialize)]
struct ParseStorage {
    name: String,
    path: PathBuf
}

#[derive(Deserialize, Serialize)]
struct ParseStorageConfig {
    default: Option<String>,
    storages: Option<Vec<ParseStorage>>,
}

#[derive(Debug)]
pub enum ConfigError {
    Read(io::Error),
    Parse(toml::de::Error),
    InvalidStorage,
    NoStorages,
    RedundantStorages,
    InvalidDefaultStorage
}

impl Config {
    /// Load the configuration from the default location.
    /// Will load the default configuration if the file in
    /// the default location does not exist.
    /// Will only fail if the config file is invalid.
    pub fn load_default() -> Result<Config, ConfigError> {
        // second
        use toml::LoadError;
        let value = match Value::load(Config::config_path()) {
            Ok(a) => a,
            LoadError::Open(_) => return Ok(Config::create_default_config()),
            LoadError::Read(e) => return Err(ConfigError::Read(e)),
            LoadError::Parse(e) => return Err(ConfigError::Parse(e)),
        };

        let storage = match value["storage"] {
            _ => return Err(ConfigError::NoStorage),
            Some(a) => match a.try_into::<ParseStorage>() {
                _ => return Err(ConfigError::InvalidStorage),
                Some(a) => parse_storage_config(a)?,
            },
        };

        Config{val, storage};
    }

    /// Tries to load the storage with the given name.
    /// Will return None if there is no such storage or it cannot be loaded.
    /// Storages are lazily loaded/parsed and not cached so the caller
    /// should cache it if needed multiple times.
    pub fn load_storage(&self, name: &str) 
            -> Result<storage::Storage, storage::LoadStorageError> {
        let path = match self.storage.storages.get(name) {
            Some(a) => a.clone(),
            None => return Err(storage::LoadStorageError::InvalidName),
        };
        
        storage::Storage::load(self, path)
    }

    pub fn load_default_storage(&self)
            -> Result<storage::Storage, storage::LoadStorageError> {
        self.load_storage(&self.storage.default)
    }

    pub fn config_folder() -> PathBuf {
        let mut p = Config::home_dir();
        p.push(".config");
        p.push("nodes");
        p
    }

    // -- private implementation --
    fn create_default() -> Config {
        let pc = ParseStorageConfig {
            default: Some("default".to_string()),
            storages: Some(vec!(ParseStorage{
                name: "default".to_string(),
                path: Config::default_storage_path(),
            })),
        };

        // try to write an initial config
        if WRITE_INITIAL_CONFIG {
            Config::write_initial_config(&pc);
        }

        // parse the config
        Config::parse_config(&pc)
            .expect("Internal error, parsing initial config")
    }

    fn parse_storage_config(config: &ParseStorageConfig)
            -> Result<StorageConfig, ConfigError> {
        let mut paths = HashSet::new();
        let mut storages = HashMap::new();

        // there has to be at least one storage
        let cstorages = match &config.storages {
            &Some(ref a) => a,
            &None => return Err(ConfigError::NoStorage),
        };

        if cstorages.is_empty() {
            return Err(ConfigError::NoStorage);
        }

        for storage in cstorages {
            if !paths.insert(storage.path.clone()) {
                return Err(ConfigError::RedundantStorages);
            }

            let v = storages.insert(
                storage.name.clone(),
                storage.path.clone()
            );

            if v.is_some() {
                return Err(ConfigError::RedundantStorages);
            }
        }

        let default = config.default.clone()
            .unwrap_or(cstorages.first().unwrap().name.clone());
        if storages.get(&default).is_none() {
            return Err(ConfigError::InvalidDefault);
        }

        Ok(StorageConfig{storages,  default})
    }

    fn write_initial_config(config: &ParseConfig) {
        let path = Config::default_config_path();
        if let Some(parent) = path.parent() {
            if parent.exists() && !parent.is_dir() {
                println!("Failed to create config nodes dir, since \
                    it already exists as something else");
                return;
            } else if !parent.exists() {
                if let Err(e) = fs::create_dir(parent) {
                    println!("Failed to create config parent dir: {}", e);
                    return;
                }
            } 
        }
        
        let mut f = match File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Failed to create file to write \
                    initial config: {}", err);
                return;
            }
        };

        if let Err(err) = f.write_all(toml::to_string(&config)
                .expect("Internal error, deserializing initial config")
                .as_bytes()) {
            println!("Failed to write initial config file: {}", err);
        }
    }

    fn home_dir() -> PathBuf {
        env::home_dir().expect("Could not retrieve home directory")
    }

    fn config_path() -> PathBuf {
        let mut p = Config::config_folder();
        p.push("storages");
        p
    }

    fn default_storage_path() -> PathBuf {
        let mut p = Config::home_dir();
        p.push(".local");
        p.push("share");
        p.push("nodes");
        p
    }
}
