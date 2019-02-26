use super::toml;
use super::config::Config;
use super::node::Node;
use std::io;
use std::fs;

use std::io::prelude::*;
use std::path::PathBuf;
use std::fs::File;

#[derive(Deserialize, Serialize)]
pub struct StorageState {
    last_id: u64,
}

pub struct Storage<'a> {
    config: &'a Config,
    name: String,
    path: PathBuf,
    state: StorageState
}

#[derive(Debug)]
pub enum LoadStorageError {
    InvalidName,
    NotFound,
    Open(io::Error),
    Read(io::Error),
    Parse(toml::de::Error)
}

impl<'a> Storage<'a> {
    /// Loads the storage for the given stoage path.
    /// Note that the passed path has to be the base path of the storage,
    /// not the storage file itself.
    pub fn load(config: &'a Config, name: &str, path: PathBuf)
            -> Result<Storage<'a>, LoadStorageError> {
        let mut spath = path.clone();
        spath.push("storage");
        let mut f = match File::open(spath) {
            Ok(f) => f,
            Err(e) => return Err(LoadStorageError::Open(e)),
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return Err(LoadStorageError::Read(e));
        }

        let state = match toml::from_str::<StorageState>(&s) {
            Ok(s) => s,
            Err(e) => return Err(LoadStorageError::Parse(e)),
        };

        Ok(Storage { config, name: name.to_string(), path, state })
    }

    /// Returns the next id that would be used for a node.
    /// Does not automatically increase it, see use_id.
    pub fn next_id(&self) -> u64 {
        self.state.last_id + 1
    }

    /// Uses the current next_id, i.e. increases the id counter.
    pub fn use_id(&mut self) {
        self.state.last_id += 1;
    }

    /// Returns the path of this storage
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the nodes path
    pub fn nodes_path(&self) -> PathBuf {
        let mut path = self.path.clone();
        path.push("nodes");
        path
    }

    /// Returns the associated config
    pub fn config(&self) -> &Config {
        self.config
    }

    /// Returns the name of this storage
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a list of all nodes in this storage.
    pub fn nodes(&self) -> Vec<Node> {
        self.list_nodes(&self.nodes_path(), false)
    }

    /// Returns a list of all nodes in this storage.
    pub fn archived(&self) -> Vec<Node> {
        let mut path = self.nodes_path();
        path.push("archive");
        self.list_nodes(&path, true)
    }

    // TODO: id inc should probably be compile-time checked
    // should be possible somehow
    pub fn next_node(&self) -> Node {
        Node::new(self, self.next_id())
    }

    // TODO: should probably return iterator?
    fn list_nodes<'b>(&'b self, path: &PathBuf, archived: bool) -> Vec<Node<'a, 'b>> {
        let dir = match fs::read_dir(path) {
            Ok(a) => a,
            Err(_) => {
                return Vec::new();
            },
        };

        let mut nodes = Vec::new();
        for entry in dir {
            let entry = match entry {
                Ok(a) => a,
                Err(e) => {
                    println!("Invalid nodes entry in storage {}: {}",
                        self.name, e);
                    continue;
                },
            };

            let entry = entry.path();
            if entry.is_dir() {
                continue;
            }

            let id = entry.file_stem()
                .and_then(|f| f.to_str())
                .and_then(|f| f.parse::<u64>().ok());

            match id {
                Some(id) => nodes.push(Node::new_archived(&self, id, archived)),
                None => println!("Invalid node file in {}: {}",
                    self.name, entry.to_str().unwrap_or("<invalid>")),
            }
        }

        nodes
    }
}

/// RAII drop implementation to save the storages state.
impl<'a> Drop for Storage<'a> {
    fn drop(&mut self) {
        let mut path = self.path.clone();
        path.push("storage");
        let mut f = match File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Failed to create file to write \
                    storage state: {}", err);
                return;
            }
        };

        if let Err(err) = f.write_all(toml::to_string(&self.state)
                .expect("Internal error, deserializing state file")
                .as_bytes()) {
            println!("Failed to write initial config file: {}", err);
        }
    }
}
