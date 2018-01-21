extern crate toml;
extern crate time;

#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use std::io::{BufRead, BufReader};

const NODESDIR: &str = "/home/nyorain/.local/share/nodes/nodes";
const METADIR: &str = "/home/nyorain/.local/share/nodes/.meta";
const STATEFILE: &str = "/home/nyorain/.local/share/nodes/.meta/state";

/// Represents a loaded toml file.
/// Uses toml-rs internally so will not preserve whitespace and value
/// order. Offers additional functionality.
// pub struct TomlFile {
//     toml: Option<toml::Value>,
//     file: PathBuf
// }

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

    // TODO
    return s.parse::<toml::Value>()
        .unwrap_or_else(|e| {
            println!("Error parsing toml: {}", e);
            toml_new()});
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

struct Node {
    id: u64,
}

// in which way to sort a node list
arg_enum!{
    #[derive(PartialEq, Debug)]
    pub enum NodeSort {
        ID, // by node id
        LA, // last accessed
        Name
    }
}


impl Node {
    fn meta_path(&self) -> PathBuf {
        let mut pb = PathBuf::from(METADIR);
        pb.push("nodes");
        pb.push(&self.id.to_string());
        pb
    }

    fn node_path(&self) -> PathBuf {
        let mut pb = PathBuf::from(NODESDIR);
        pb.push(&self.id.to_string());
        pb
    }
}

pub fn command_add(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let node = Node {id: state.next_id()};

    // copy file
    let fname = args.value_of("file").unwrap();
    let path = Path::new(fname);
    if let Err(e) = fs::copy(path, node.node_path()) {
        println!("Could not add node {}: {}", fname, e);
        return -1;
    }

    // create meta file
    state.use_id();
    let name = match args.value_of("name") {
        Some(name) => name.to_string(),
        None => node.id.to_string()
    };

    let mut meta = toml_new();
    let now = time::now().rfc3339().to_string();
    toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));
    save_toml_file(&meta, node.meta_path());

    state.set("last.created", toml::Value::Integer(node.id as i64));
    state.set("last.accessed", toml::Value::Integer(node.id as i64));

    // output information
    if args.is_present("name") {
        println!("Created Node {}: {}", node.id, name);
    } else {
        println!("Created Node {}", node.id);
    }

    0
}

pub fn command_rm(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = Node {id};

    if let Err(e) = fs::remove_file(node.node_path()) {
        println!("Could not remove node {}: {}", id, e);
        return -1;
    };

    if let Err(e) = fs::remove_file(node.meta_path()) {
        println!("Error removing meta file: {}: {}", id, e);
    }

    println!("Removed node {}", id);
    0
}

/// Trims the given string to the length max_length.
/// The last three chars will be "..." if the string was longer
/// than max_length.
fn short_string(lstr: &str, max_length: usize) -> String {
    let mut too_long = false;
    let mut s = String::new();
    let mut append = String::new();

    for (i, c) in lstr.chars().enumerate() {
        if i == max_length {
            too_long = true;
            break;
        } else if i >= max_length - 3 {
            append.push(c);
        } else {
            s.push(c);
        }
    }

    s.push_str(if too_long { "..." } else { append.as_str() });
    s
}

fn read_summary(path: &PathBuf) -> String {
    let f = match File::open(path) {
        Ok(v) => v,
        Err(_) => return "".to_string(),
    };

    let f = BufReader::new(&f);
    let line = match f.lines().next() {
        Some(v) => v,
        _ => return "".to_string(),
    };

    let line = match line {
        Ok(v) => v,
        Err(_) => return "".to_string(),
    };

    line
}

pub fn command_create(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let node = Node {id: state.next_id()};

    // create node
    let path = node.node_path();

    if let Some(content) = args.value_of("content") {
        let mut f = File::create(path).expect("Invalid name");
        f.write_all(content.as_bytes()).expect("Failed to write node");
    } else {
        // TODO: use spawn?
        let pname = path.to_str().expect("Invalid name");
        let mut child = Command::new("nvim").arg(pname).spawn().expect("spawn");
        child.wait().expect("wait");

        // if node was not written, there is nothing more to do here
        if !path.exists() {
            println!("No node was created");
            return -1;
        }
    }

    state.use_id();
    let name = match args.value_of("name") {
        Some(name) => name.to_string(),
        None => "".to_string()
    };

    // set meta data
    let mut meta = toml_new();
    let now = time::now().rfc3339().to_string();
    let nodetype = args.value_of("type").unwrap_or("text");

    toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));
    toml_set(&mut meta, "type", toml::Value::from(nodetype.to_string()));

    if let Some(tags) = args.values_of("tags") {
        let mut collected: Vec<toml::Value> = Vec::new();
        for tag in tags {
            collected.append(&mut tag.split_whitespace()
                .map(|x| toml::Value::String(x.to_string()))
                .collect());
        }
        toml_set(&mut meta, "tags", toml::Value::Array(collected));
    }

    save_toml_file(&meta, node.meta_path());

    state.set("last.created", toml::Value::Integer(node.id as i64));
    state.set("last.accessed", toml::Value::Integer(node.id as i64));

    // output information
    if args.is_present("name") {
        println!("Created Node {}: {}", node.id, name);
    } else {
        println!("Created Node {}", node.id);
    }

    0
}

fn nodes_list() -> Vec<u64> {
    fs::read_dir(NODESDIR).unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| !p.is_dir())
        .map(|p| -> u64 { p
            .file_stem().unwrap()
            .to_str().unwrap()
            .parse().unwrap()
        }).collect()
}

struct FoundNode {
    id: u64,
    name: String,
    summary: String,
    accessed: time::Tm
}

const NAME_SIZE: usize = 20;
const SUMMARY_SIZE: usize = 50;

pub fn command_ls(args: &clap::ArgMatches) -> i32 {
    let name = args.value_of("name");
    let tag = args.value_of("tag");
    let sort = value_t!(args, "sort", NodeSort).unwrap_or_else(|e| e.exit());
    let mut num = value_t!(args, "num", u64).unwrap_or_else(|e| e.exit());
    let mut nodes: Vec<FoundNode> = Vec::new();

    for id in nodes_list() {
        let node = Node {id};
        let meta = parse_toml_file(node.meta_path());
        let nname = toml_get(&meta, "name").unwrap().as_str().unwrap();

        // check if node has name
        if let Some(name) = name {
            if name != nname {
                continue;
            }
        }

        // check for tag
        if let Some(tag) = tag {
            let mut found = false;
            let ntags = toml_get(&meta, "tags")
                .and_then(|t| t.as_array()); // (optional) tags array

            if let Some(nntags) = ntags { // if there are tags for node
                for ntag in nntags { // for every tag
                    if let Some(ntag) = ntag.as_str() { // if tag is string
                        if ntag == tag {
                            found = true;
                            break;
                        }
                    } else { // we have non-tag string
                        println!("Invalid tag type {} for node {}",
                            ntag.type_str(), id);
                    }
                }
            }

            if !found {
                continue;
            }
        }

        let summary = read_summary(&node.node_path());
        let summary = short_string(summary.as_str(), SUMMARY_SIZE);

        // accessed
        let accessed = time::strptime(
            toml_get(&meta, "accessed").unwrap().
            as_str().unwrap(), "%Y-%m-%dT%H:%M:%S").unwrap(); // rfc3339
        let node = FoundNode {
            id,
            name: short_string(nname, NAME_SIZE),
            summary: short_string(summary.as_str(), SUMMARY_SIZE),
            accessed,
        };
        nodes.push(node);
    }

    match sort {
        NodeSort::ID => nodes.sort_by_key(|v| v.id),
        NodeSort::LA => nodes.sort_by_key(|v| v.accessed),
        NodeSort::Name => nodes.sort_by_key(|v| v.name.clone()), // TODO
    };

    if !args.is_present("reverse") {
        nodes.reverse();
    }

    for node in nodes {
        if num == 0 {
            break;
        }

        println!("{}:\t{:<w2$}    {:<w3$}",
            node.id, node.name, node.summary,
            w2 = NAME_SIZE, w3 = SUMMARY_SIZE);
        num -= 1;
    }

    0
}

fn output(path: &PathBuf) -> std::io::Result<()> {
    let f = File::open(path)?;

    let f = BufReader::new(&f);
    for line in f.lines() {
        let lline = match line {
            Ok(v) => v,
            _ => "<Invalid line>".to_string(),
        };

        println!("{}", lline);
    }

    Ok(())
}

const DEFAULT_PROGRAM: &str = "nvim";
fn spawn(state: &State, category: &str, ntype: &str, path: &PathBuf) {
    let mut cat = String::from("programs.");
    cat.push_str(&category);
    cat.push('.');
    cat.push_str(ntype);

    // TODO: handle prog array
    let prog = state.get(&cat)
        .map(|v| v.as_str().unwrap_or(DEFAULT_PROGRAM))
        .unwrap_or(DEFAULT_PROGRAM);

    // handle special values
    if prog == "!output" {
       output(path).unwrap_or_else(|e| println!("output error {}", e));
       return;
    }
    
    // try to execute program
    Command::new(prog).arg(path.to_str().unwrap())
        .spawn().expect("Failed to spawn program")
        .wait().expect("Failed to wait for program");
}

pub fn command_edit(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = Node {id};

    let mut state = State::load();

    let meta = args.is_present("meta");
    let path = if meta  {
        node.meta_path()
    } else {
        node.node_path()
    };

    if !path.exists() {
        println!("Node {} does not exist", id);
        return -1;
    }

    // edit meta file
    if meta {
        spawn(&state, "edit", "text", &path);
        return 0;
    }

    let mut meta = parse_toml_file(node.meta_path());

    {
        let nodetype = toml_get(&meta, "type")
            .map(|v| v.as_str().unwrap_or("text"))
            .unwrap_or("text");
        spawn(&state, "edit", nodetype, &path);
    }

    state.set("last.accessed", toml::Value::Integer(id as i64));

    let now = time::now().rfc3339().to_string();
    toml_set(&mut meta, "accessed", toml::Value::from(now));
    save_toml_file(&meta, node.meta_path());

    0
}

pub fn command_show(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = Node {id};

    let mut state = State::load();

    let meta = args.is_present("meta");
    let path = if meta  {
        node.meta_path()
    } else {
        node.node_path()
    };

    if !path.exists() {
        println!("Node {} does not exist", id);
        return -1;
    }

    // edit meta file
    if meta {
        spawn(&state, "show", "text", &path);
        return 0;
    }

    let mut meta = parse_toml_file(node.meta_path());

    {
        let nodetype = toml_get(&meta, "type")
            .map(|v| v.as_str().unwrap_or("text"))
            .unwrap_or("text");
        spawn(&state, "show", nodetype, &path);
    }

    state.set("last.accessed", toml::Value::Integer(id as i64));

    let now = time::now().rfc3339().to_string();
    toml_set(&mut meta, "accessed", toml::Value::from(now));
    save_toml_file(&meta, node.meta_path());

    0
}

pub fn command_open_state(_: &clap::ArgMatches) -> i32 {
    Command::new("nvim").arg(STATEFILE)
        .spawn().expect("spawn")
        .wait().expect("wait");
    0
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
