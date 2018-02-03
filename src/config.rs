use super::toml;
use super::storage;

use std::io;
use std::env;

use std::path::PathBuf;
use std::collections::HashSet;
use std::collections::HashMap;
use toml::ValueImpl;

pub struct StorageConfig {
    default: String,
    storages: HashMap<String, PathBuf>
}

pub struct Config {
    value: Option<toml::Value>,
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
    NoStorage,
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
        let value = match toml::Value::load(Config::config_path()) {
            Ok(a) => a,
            Err(LoadError::Open(_)) => return Ok(Config::default_config()),
            Err(LoadError::Read(e)) => return Err(ConfigError::Read(e)),
            Err(LoadError::Parse(e)) => return Err(ConfigError::Parse(e)),
        };

        let storage = match value.get("storage") {
            None => return Err(ConfigError::NoStorage),
            Some(a) => match &a.clone().try_into::<ParseStorageConfig>() {
                &Ok(ref a) => Config::parse_storage_config(a)?,
                _ => return Err(ConfigError::InvalidStorage),
            },
        };

        Ok(Config{value: Some(value), storage})
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
        
        storage::Storage::load(self, name, path)
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

    /// Returns the parsed config file as value
    pub fn value(&self) -> &Option<toml::Value> {
        &self.value
    }

    // -- private implementation --
    fn default_config() -> Config {
        let mut storages = HashMap::new();
        storages.insert("default".to_string(),
            Config::default_storage_path());
        Config {
            value: None,
            storage: StorageConfig {
                default: "default".to_string(),
                storages,
            }
        }
    }

    fn parse_storage_config(config: &ParseStorageConfig)
            -> Result<StorageConfig, ConfigError> {
        let mut paths = HashSet::new();
        let mut storages = HashMap::new();

        // there has to be at least one storage
        let cstorages = match &config.storages {
            &Some(ref a) => a,
            &None => return Err(ConfigError::NoStorages),
        };

        if cstorages.is_empty() {
            return Err(ConfigError::NoStorages);
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

        // just use the first entry as default if there is none given
        let default = config.default.clone()
            .unwrap_or(cstorages.first().unwrap().name.clone());
        if storages.get(&default).is_none() {
            return Err(ConfigError::InvalidDefaultStorage);
        }

        Ok(StorageConfig{storages, default})
    }

    fn home_dir() -> PathBuf {
        env::home_dir().expect("Could not retrieve home directory")
    }

    fn config_path() -> PathBuf {
        let mut p = Config::config_folder();
        p.push("config");
        p
    }

    fn default_storage_path() -> PathBuf {
        let mut p = Config::home_dir();
        p.push(".local");
        p.push("share");
        p.push("nodes-dummy"); // TODO
        p
    }
}
