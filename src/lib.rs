extern crate time;
extern crate toml;
extern crate regex;

#[macro_use] extern crate clap;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate nom;

pub mod parse;
pub mod pattern;
pub mod pattern2;
pub mod tree;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use std::iter::Iterator;
use std::io::{BufRead, BufReader};
use std::io;

const NODESDIR: &str = "/home/nyorain/.local/share/nodes/nodes";
const METADIR: &str = "/home/nyorain/.local/share/nodes/.meta";
const STATEFILE: &str = "/home/nyorain/.local/share/nodes/.meta/state";

/// Represents the node state.
pub struct State {
    toml: parse::Value
}

impl State {
    /// Creates a new state from the default state file.
    pub fn load() -> State {
        let v = parse::Value::parse(STATEFILE)
            .expect("Invalid statefile");
        State { toml: v}
    }

    /// Returns a new id for a new node.
    /// Does not automatically increase it, see use_id
    pub fn next_id(&mut self) -> u64 {
        let idv = self.toml.get("id").expect("No id in state");
        let id = idv.as_integer().expect("Invalid id in state");
        (id + 1) as u64
    }

    /// Uses the current next_id.
    /// Increases the id and sets the next_id as lastly created.
    pub fn use_id(&mut self) {
        let idv = self.toml.get_mut("id").expect("No id in state");
        let id = idv.as_integer().expect("Invalid id in state");
        *idv = toml::Value::Integer(id + 1);
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.toml.save(STATEFILE).expect("Failed to save state");
    }
}

struct Node {
    id: u64
}

impl Node {
    /// Returns a new node for the given id.
    /// Does not parse the meta file yet.
    fn new(id: u64) -> Node {
        Node { id }
    }

    /// Returns the parsed meta toml value.
    fn meta(&self) -> parse::Value {
        parse::Value::parse(self.meta_path())
            .unwrap_or(parse::Value::new())
    }

    /// Returns the path of node file.
    /// Does not guarantee it exists.
    fn node_path(&self) -> PathBuf {
        let mut pb = PathBuf::from(NODESDIR);
        pb.push(&self.id.to_string());
        pb
    }

    /// Returns whether the node exists.
    fn exists(&self) -> bool {
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

    /// Returns the path of the nodes meta file.
    /// Does not guarantee it exists.
    fn meta_path(&self) -> PathBuf {
        let mut pb = PathBuf::from(METADIR);
        pb.push("nodes");
        pb.push(&self.id.to_string());
        pb
    }

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

pub fn command_add(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let mut node = Node::new(state.next_id());

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

    let now = time::now().rfc3339().to_string();
    let mut meta = node.meta();
    meta.set("name", toml::Value::from(name.clone()));
    meta.set("created", toml::Value::from(now.clone()));
    meta.set("changed", toml::Value::from(now.clone()));
    meta.set("accessed", toml::Value::from(now.clone()));

    state.toml.set("last.created", toml::Value::Integer(node.id as i64));
    state.toml.set("last.accessed", toml::Value::Integer(node.id as i64));

    meta.save(node.meta_path()).expect("Failed to save node meta file");

    // output information
    if args.is_present("name") {
        println!("Created Node {}: {}", node.id, name);
    } else {
        println!("Created Node {}", node.id);
    }

    0
}

pub fn remove_node(id: u64) -> io::Result<()> {
    let node = Node::new(id);
    fs::remove_file(node.node_path())?;
    fs::remove_file(node.meta_path())?;
    Ok(())
}

pub fn command_rm(args: &clap::ArgMatches) -> i32 {
    let ids = values_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let mut res = 0;
    for id in ids {
        if remove_node(id).is_err() {
            res += 1;
        }
    }

    res
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

fn read_node(path: &PathBuf, mut lines: u64, dot: bool) -> String {
    let f = match File::open(path) {
        Ok(v) => v,
        Err(_) => return "<Invalid node>".to_string(),
    };

    let f = BufReader::new(&f);
    let mut ret = String::new();

    for line in f.lines().take(lines as usize) {
        if lines == 0 {
            if dot {
                ret.push_str("[...]");
            }
            break;
        }

        if let Ok(l) = line {
            ret.push_str(&l);
            if dot { // TODO: extra param?
                ret.push('\n');
            }
        }

        lines -= 1;
    }

    ret
}

pub fn command_create(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let mut node = Node::new(state.next_id());

    // create node
    let path = node.node_path();

    if let Some(content) = args.value_of("content") {
        let mut f = File::create(path).expect("Invalid name");
        f.write_all(content.as_bytes()).expect("Failed to write node");
    } else {
        // TODO: use spawn?
        let pname = path.to_str().expect("Invalid name");
        Command::new("nvim").arg(pname)
            .spawn().expect("spawn")
            .wait().expect("wait");

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
    let mut meta = node.meta();
    let nodetype = args.value_of("type").unwrap_or("text");
    let now = time::now().rfc3339().to_string();

    meta.set("name", toml::Value::from(name.clone()));
    meta.set("created", toml::Value::from(now.clone()));
    meta.set("changed", toml::Value::from(now.clone()));
    meta.set("accessed", toml::Value::from(now.clone()));

    state.toml.set("last.created", toml::Value::Integer(node.id as i64));
    state.toml.set("last.accessed", toml::Value::Integer(node.id as i64));

    if let Some(tags) = args.values_of("tags") {
        let mut collected: Vec<toml::Value> = Vec::new();
        for tag in tags {
            collected.append(&mut tag.split_whitespace()
                .map(|x| toml::Value::String(x.to_string()))
                .collect());
        }
        meta.set("tags", toml::Value::Array(collected));
    }

    meta.save(node.meta_path()).expect("Failed to save node meta file");

    state.toml.set("last.created", node.id as i64);
    state.toml.set("last.accessed", node.id as i64);

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

fn node_summary(path: &PathBuf, lines: u64) -> String {
    if lines == 1 {
        short_string(&read_node(&path, lines, false), SUMMARY_SIZE)
    } else {
        read_node(&path, lines, true)
    }
}

fn list_node(node: &FoundNode, lines: u64) {
    if lines == 1 {
        println!("{}:\t{:<w2$}    {:<w3$}",
            node.id, short_string(&node.name, NAME_SIZE), node.summary,
            w2 = NAME_SIZE, w3 = SUMMARY_SIZE);
    } else {
        println!("{}:\t{}", node.id, node.name);
        for line in node.summary.lines() {
            println!("\t{}", line);
        }
        println!();
    }
}

pub fn command_ls(args: &clap::ArgMatches) -> i32 {
    let name = args.value_of("name");
    let tag = args.value_of("tag");
    let sort = value_t!(args, "sort", NodeSort).unwrap_or(NodeSort::ID);
    let mut lines = value_t!(args, "lines", u64).unwrap_or(1);

    if args.is_present("full") {
        lines = 10000; // TODO, we can do better than this!
    }

    let mut num = value_t!(args, "num", usize).unwrap_or(10);
    let mut nodes: Vec<FoundNode> = Vec::new();

    for id in nodes_list() {
        let mut node = Node::new(id);
        let mut meta = node.meta();
        let nname = meta.get("name").unwrap().as_str().unwrap();

        // check if node has name
        if let Some(name) = name {
            if name != nname {
                continue;
            }
        }

        // check for tag
        if let Some(tag) = tag {
            let mut found = false;
            let ntags = meta.get("tags")
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

        let summary = node_summary(&node.node_path(), lines);

        // accessed
        let accessed = meta.get("accessed").unwrap().
            as_str().unwrap();
        let accessed = time::strptime(accessed, "%Y-%m-%dT%H:%M:%S")
            .unwrap(); // rfc3339
        let node = FoundNode {
            id,
            name: nname.to_string(),
            summary: summary,
            accessed,
        };

        nodes.push(node);
    }

    match sort {
        NodeSort::ID => nodes.sort_by_key(|v| v.id),
        NodeSort::LA => nodes.sort_by_key(|v| v.accessed),
        NodeSort::Name => nodes.sort_by_key(|v| v.name.clone()), // TODO
    };

    if !args.is_present("reverse") { // TODO: currently inversed
        nodes.reverse();
    }

    nodes.truncate(num);

    if args.is_present("reverse_list") {
        nodes.reverse();
    }

    for node in nodes {
        list_node(&node, lines);
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

const DEFAULT_PROGRAM: &str = "nvim"; // TODO: don't hardcode this
fn spawn(state: &State, category: &str, ntype: &str, path: &PathBuf) {
    let mut cat = String::from("programs.");
    cat.push_str(&category);
    cat.push('.');
    cat.push_str(ntype);

    let prog = state.toml.get(&cat);

    // TODO: also split whitespace?
    // to make something like 'editor -i' work?
    let mut prog = match prog {
        Some(val) => {
            match val {
                &toml::Value::String(ref p) => vec!(p.clone()),
                &toml::Value::Array(ref p) => {
                    let mut v: Vec<String> = Vec::new();

                    // TODO: also allow non-string values?
                    for val in p {
                        if let Some(arg) = val.as_str() {
                            v.push(arg.to_string());
                        } else {
                            println!("Non-string program arg!");
                        }
                    }
                    v
                }
                _ => {
                    println!("Invalid program spec type");
                    vec!(DEFAULT_PROGRAM.to_string())
                }
            }
        },
        _ => vec!(DEFAULT_PROGRAM.to_string())
    };

    for arg in prog.iter_mut() {
        while let Some(p) = arg.find("@content") {
            arg.drain(p..p+8);

            let mut s = String::new();
            let f = File::open(path).unwrap().read_to_string(&mut s);

            arg.insert_str(p, &s);
        }
    }

    // handle special values
    if prog.len() == 1 && prog[0] == "!output" {
       output(path).unwrap_or_else(|e| println!("output error {}", e));
       return;
    }

    // TODO: maybe not as expected
    // e.g. when a user sets ['editor', '-i'] as program
    // we should probably check if at least one meta-value (e.g. @content)
    // was used
    if prog.len() == 1 {
        prog.push(path.to_str().unwrap().to_string());
    }

    println!("Executing: {:?}", prog);

    // try to execute program
    Command::new(&prog[0]).args(prog[1..].iter())
        .spawn().expect("Failed to spawn program")
        .wait().expect("Failed to wait for program");
}

pub fn command_edit(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let mut node = Node::new(id);
    let mut state = State::load();

    let mut meta = args.is_present("meta");
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

    let mut meta = node.meta();

    {
        let nodetype = meta.get("type")
            .map(|v| v.as_str().unwrap_or("text"))
            .unwrap_or("text");
        spawn(&state, "edit", nodetype, &path);
    }

    state.toml.set("last.accessed", id as i64);

    let now = time::now().rfc3339().to_string();
    meta.set("accessed", now);
    meta.save(node.meta_path()).expect("Failed to save node meta file");

    0
}

pub fn command_show(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let mut node = Node::new(id);
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

    let mut meta = node.meta();

    {
        let nodetype = meta.get("type")
            .map(|v| v.as_str().unwrap_or("text"))
            .unwrap_or("text");
        spawn(&state, "show", nodetype, &path);
    }

    state.toml.set("last.accessed", id as i64);

    let now = time::now().rfc3339().to_string();
    meta.set("accessed", now);
    meta.save(node.meta_path()).expect("Failed to save node meta file");

    0
}

pub fn command_open_state(_: &clap::ArgMatches) -> i32 {
    Command::new("nvim").arg(STATEFILE)
        .spawn().expect("spawn")
        .wait().expect("wait");
    0
}

pub fn command_dev(args: &clap::ArgMatches) -> i32 {
    /*
    let mut tree = pattern::NodePred::new();
    let root = tree.add_root(pattern::PredNode::Not);
    tree.add(root, pattern::PredNode::Pred(pattern::Pred {
        entry: "tags".to_string(),
        pred_type: pattern::PredType::Matches("todo".to_string())
    }));
    */

    let p = args.value_of("pattern").unwrap();
    // let tree = pattern::parse_pattern(p).unwrap();
    let tree = match pattern2::parse_condition(p) {
        Ok(a) => a,
        Err(err) => {
            println!("{}", err);
            return -1;
        },
    };

    pattern2::print_cond(&tree);
    println!();

    let num = 100;
    let lines = 1;
    let mut nodes: Vec<FoundNode> = Vec::new();

    for id in nodes_list() {
        let mut node = Node::new(id);
        let mut meta = node.meta();
        let nname = meta.get("name").unwrap().as_str().unwrap();

        // check predicate
        // if !pattern::node_matches(&meta.toml, &tree) {
        //     continue;
        // }
        if !pattern2::node_matches(&meta.toml, &tree) {
            continue;
        }

        // push node and data
        let summary = node_summary(&node.node_path(), lines);
        let accessed = meta.get("accessed").unwrap().
            as_str().unwrap();
        let accessed = time::strptime(accessed, "%Y-%m-%dT%H:%M:%S")
            .unwrap(); // rfc3339
        let node = FoundNode {
            id,
            name: nname.to_string(),
            summary: summary,
            accessed,
        };

        nodes.push(node);
    }

    nodes.sort_by_key(|v| v.id);
    nodes.reverse();
    nodes.truncate(num);

    for node in nodes {
        list_node(&node, lines);
    }

    0
}
