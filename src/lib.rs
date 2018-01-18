extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

const STATEFILE: &str = "/home/nyorain/.local/share/nodes/.meta/state";

/// Represents a loaded toml file.
/// Uses toml-rs internally so will not preserve whitespace and value
/// order. Offers additional functionality.
pub struct TomlFile {
    toml: Option<toml::Value>,
    file: PathBuf
}

/// Represents the node state.
pub struct State {
    toml: toml::Value,
}

impl State {
    /// Creates a new state from the default state file.
    pub fn load() -> State {
         State {
             toml: parse_toml_file(STATEFILE)
         }
    }

    /// Saves the state to file.
    /// Will overwrite it.
    pub fn save(&self) {
        save_toml_file(&self.toml, STATEFILE);
    }

    /// toml_get/set utility
    pub fn get_mut(&mut self, name: &str) -> Option<&mut toml::Value> {
        toml_get_mut(&mut self.toml, name)
    }

    pub fn get(&self, name: &str) -> Option<&toml::Value> {
        toml_get(&self.toml, name)
    }

    pub fn set(&mut self, name: &str, val: toml::Value) -> bool {
        toml_set(&mut self.toml, name, val)
    }

    /// Returns a new id for a new node.
    /// Does not automatically increase it, see use_id
    pub fn next_id(&mut self) -> u64 {
        let idv = self.get("id").expect("No id in state");
        let id = idv.as_integer().expect("Invalid id in state");
        (id + 1) as u64
    }

    /// Uses the current next_id.
    /// Increases the id and sets the next_id as lastly created.
    pub fn use_id(&mut self) {
        let idv = self.get_mut("id").expect("No id in state");
        let id = idv.as_integer().expect("Invalid id in state");
        *idv = toml::Value::Integer(id + 1);
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.save();
    }
}

/// Parses the toml file at the given path and returns the
/// toplevel toml value
pub fn parse_toml_file<P: AsRef<Path>>(path: P) -> toml::Value {
    let mut f = File::open(path).expect("Failed to open toml file");
    let mut s = String::new();
    f.read_to_string(&mut s).expect("Failed to read toml file");
    return s.parse::<toml::Value>().unwrap();
}

/// Saves the given toml value to a file at the given path.
pub fn save_toml_file<P: AsRef<Path>>(val: &toml::Value, p: P) {
    let mut f = File::create(p).expect("Failed to open state file");
    let str = toml::ser::to_string_pretty(val).unwrap();
    f.write_all(str.as_bytes()).unwrap();
}

/// Returns an empty toml value as table
pub fn toml_new() -> toml::Value {
    toml::Value::from(toml::value::Table::new())
}

// TODO: allow array access in name. Like foo.bar.arr.3
/// Returns the value with the given name, if existent.
/// Can access sub tables, like foo.bar.val
pub fn toml_get<'a>(v: &'a toml::Value, name: &str) 
        -> Option<&'a toml::Value> {
    let mut next = v;
    for part in name.split('.') {
        let cur = v;
        match *cur {
            toml::Value::Table(ref table) =>
                match table.get(part) {
                    Some(entry) => next = entry,
                    None => return None,
                },
            _ => return None,
        };
    }
    Some(next)
}

pub fn toml_get_mut<'a>(v: &'a mut toml::Value, name: &str) 
        -> Option<&'a mut toml::Value> {
    let mut next = v;
    for part in name.split('.') {
        let cur = next;
        match *cur {
            toml::Value::Table(ref mut table) =>
                match table.get_mut(part) {
                    Some(entry) => next = entry,
                    None => return None,
                },
            _ => return None,
        };
    }
    Some(next)
}

/// Returns false if name cannot be inserted.
/// E.g. when name is "foo.bar" but "foo" is already a (non-table)
/// value.
pub fn toml_set(v: &mut toml::Value, name: &str, val: toml::Value) -> bool {
    let mut next = v;
    let mut it = name.split('.');
    let last = it.next_back().expect("Invalid name given");

    // make sure all sub tables exist, create them if needed
    for part in it {
        let cur = next;
        match *cur {
            toml::Value::Table(ref mut table) => {
                let e = table.entry(part.to_string());
                next = e.or_insert(toml_new());
            },
            _ => return false,
        };
    }

    // insert
    match *next {
        toml::Value::Table(ref mut table) =>
            table.insert(last.to_string(), val),
        _ => return false,
    };

    true
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
