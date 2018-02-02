extern crate toml;

pub use self::toml::*;

use std::path::Path;
use std::fs::File;
use std::io;
use std::io::prelude::*;

pub enum LoadError {
    Open(io::Error),
    Read(io::Error),
    Parse(de::Error)
}

pub trait ValueImpl {
    fn new() -> Self;
    fn load<P: AsRef<Path>>(p: P) -> Result<Value, LoadError>;
    fn save<P: AsRef<Path>>(&self, p: P) -> io::Result<()>;
    fn find(&self, name: &str) -> Option<&Value>;
    fn find_mut(&mut self, name: &str) -> Option<&mut Value>;
    fn set<V: Into<Value>>(&mut self, name: &str, v: V) -> bool;
}

impl ValueImpl for Value {
    /// Creates a new empty toml table value.
    fn new() -> Value {
        Value::from(value::Table::new())
    }

    /// Tries to reads and parse the given file.
    fn load<P: AsRef<Path>>(p: P) -> Result<Value, LoadError> {
        let mut f = match File::open(p) {
            Ok(f) => f,
            Err(e) => return Err(LoadError::Open(e)),
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return Err(LoadError::Read(e));
        }

        match s.parse::<Value>() {
            Ok(s) => Ok(s),
            Err(e) => Err(LoadError::Parse(e)),
        }
    }

    fn save<P: AsRef<Path>>(&self, p: P) -> io::Result<()> {
        let s = toml::ser::to_string_pretty(&self).unwrap();
        File::create(p)?.write_all(s.as_bytes())
    }

    fn find(&self, name: &str) -> Option<&Value> {
        toml_find(self, name)
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut Value> {
        toml_find_mut(self, name)
    }

    fn set<V: Into<Value>>(&mut self, name: &str, v: V) -> bool {
        toml_set(self, name, v.into())
    }
}

// TODO: allow array access in name. Like foo.bar.arr:3

/// Returns the value with the given name, if existent.
/// Can access sub tables, like foo.bar.val
pub fn toml_find<'a>(v: &'a Value, name: &str)
        -> Option<&'a Value> {
    let mut next = v;
    for part in name.split('.') {
        let cur = next;
        match *cur {
            Value::Table(ref table) =>
                match table.get(part) {
                    Some(entry) => next = entry,
                    None => return None,
            }, _ => return None,
        };
    }
    Some(next)
}

pub fn toml_find_mut<'a>(v: &'a mut Value, name: &str)
        -> Option<&'a mut Value> {
    let mut next = v;
    for part in name.split('.') {
        let cur = next;
        match *cur {
            Value::Table(ref mut table) =>
                match table.get_mut(part) {
                    Some(entry) => next = entry,
                    None => return None,
            }, _ => return None,
        };
    }
    Some(next)
}

/// Returns false if name cannot be inserted.
/// E.g. when name is "foo.bar" but "foo" is already a (non-table)
/// value.
pub fn toml_set(v: &mut Value, name: &str, val: Value) -> bool {
    let mut next = v;
    let mut it = name.split('.');
    let last = it.next_back().expect("Invalid name given");

    // make sure all sub tables exist, create them if needed
    for part in it {
        let cur = next;
        match *cur {
            Value::Table(ref mut table) => {
                let e = table.entry(part.to_string());
                next = e.or_insert(Value::new());
            },
            _ => return false,
        };
    }

    // insert
    match *next {
        Value::Table(ref mut table) =>
            table.insert(last.to_string(), val),
        _ => return false,
    };

    true
}
