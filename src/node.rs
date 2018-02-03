use super::storage::Storage;
use super::toml;

use std::path::PathBuf;

pub struct Node<'a, 'b: 'a> {
    storage: &'a Storage<'b>,
    id: u64
}

impl<'a, 'b> Node<'a, 'b> {
    pub fn new(storage: &'a Storage<'b>, id: u64) -> Node<'a, 'b> {
        Node{storage, id}
    }

    /// Returns the nodes id.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the parsed meta toml value.
    pub fn load_meta(&self) -> Result<toml::Value, toml::LoadError> {
        <toml::Value as toml::ValueImpl>::load(self.meta_path())
    }

    /// Returns the path of node file.
    /// Does not guarantee it exists.
    pub fn node_path(&self) -> PathBuf {
        let mut pb = self.storage.path().clone();
        pb.push("nodes");
        pb.push(&self.id.to_string());
        pb
    }

    /// Returns the path of the nodes meta file.
    /// Does not guarantee it exists.
    pub fn meta_path(&self) -> PathBuf {
        let mut pb = self.storage.path().clone();
        pb.push("meta");
        pb.push(&self.id.to_string());
        pb
    }

    /// Returns whether the node exists.
    pub fn exists(&self) -> bool {
        if self.node_path().exists() {
            if self.meta_path().exists() {
                true
            } else {
                println!("Node {} has no meta file", self.id);
                false
            }
        } else {
            if self.node_path().exists() {
                println!("Meta file for non-existent node {}", self.id);
            }
            false
        }
    }

    /// Returns the associates storage
    pub fn storage(&self) -> &Storage<'b> {
        self.storage
    }
}

