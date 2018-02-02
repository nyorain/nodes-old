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
    last_accessed: u64,
}

pub struct Storage<'a> {
    config: &'a Config,
    path: PathBuf,
    state: StorageState
}

#[derive(Debug)]
pub enum LoadStorageError {
    InvalidName,
    Open(io::Error),
    Read(io::Error),
    Parse(toml::de::Error)
}

impl<'b> Storage<'b> {
    /// Loads the storage for the given stoage path.
    /// Note that the passed path has to be the base path of the storage,
    /// not the storage file itself.
    pub fn load(config: &Config, path: PathBuf) 
            -> Result<Storage, LoadStorageError> {
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

        Ok(Storage { config, path, state })
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

    // TODO: error handling
    /// Returns a list of all nodes in this storage.
    pub fn nodes(&self) -> Vec<Node> {
        fs::read_dir(self.nodes_path()).unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| !p.is_dir())
            .map(|p| -> Node { 
                Node::new(
                    &self,  
                    p.file_stem().unwrap()
                        .to_str().unwrap()
                        .parse().unwrap()
            )
        }).collect()
    }

    // TODO: id inc should probably be compile-time checked
    // should be possible somehow
    pub fn next_node(&self) -> Node {
        Node::new(self, self.next_id())
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
