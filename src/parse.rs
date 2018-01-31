extern crate toml;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;
use std::io;

/// Represents a parsed toml value.
pub struct Value {
    pub toml: toml::Value
}

impl Value {
    /// Creates a new empty toml table value.
    pub fn new() -> Value {
        Value { toml: toml::Value::from(toml::value::Table::new()) }
    }

    /// Tries to reads and parse the given file.
    pub fn parse<P: AsRef<Path>>(p: P) -> Result<Value, Box<Error>> {
        let mut s = String::new();
        File::open(p)?.read_to_string(&mut s)?;
        Ok(Value { toml: s.parse::<toml::Value>()? })
    }

    pub fn save<P: AsRef<Path>>(&self, p: P) -> io::Result<()> {
        let s = toml::ser::to_string_pretty(&self.toml).unwrap();
        File::create(p)?.write_all(s.as_bytes())
    }

    pub fn get(&self, name: &str) -> Option<&toml::Value> {
        toml_get(&self.toml, name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut toml::Value> {
        toml_get_mut(&mut self.toml, name)
    }

    pub fn set<V: Into<toml::Value>>(&mut self, name: &str, v: V) -> bool {
        toml_set(&mut self.toml, name, v.into())
    }
}

pub fn toml_new() -> toml::Value {
    toml::Value::from(toml::value::Table::new())
}

// TODO: allow array access in name. Like foo.bar.arr:3

/// Returns the value with the given name, if existent.
/// Can access sub tables, like foo.bar.val
pub fn toml_get<'a>(v: &'a toml::Value, name: &str)
        -> Option<&'a toml::Value> {
    let mut next = v;
    for part in name.split('.') {
        let cur = next;
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
