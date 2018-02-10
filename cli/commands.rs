extern crate time;

use super::clap;
use super::nodes;
use super::regex;

use nodes::toml;
use nodes::pattern;
use nodes::toml::ValueImpl;

use std::io;
use std::env;
use std::fs;
use std::process;

use std::io::BufReader;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;

const DEFAULT_NODE_TYPE: &str = "text";
const SUMMARY_SIZE: usize = 40;
const LS_COUNT_DEFAULT: usize = 30;

pub fn create(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    {
        let node_type = args.value_of("type").unwrap_or(DEFAULT_NODE_TYPE);
        let node = storage.next_node();

        let mut meta = toml::Value::new();
        if let Some(val) = args.value_of("meta") {
            let mut val = val.replace(";", "\n");
            parse_meta(&val, &mut meta);
        }

        if let Some(content) = args.value_of("content") {
            if content.is_empty() {
                println!("No content given");
                return -1;
            }

            let res = File::create(node.node_path())
                .and_then(|mut f| f.write_all(content.as_bytes()));
            if let Err(err) = res {
                println!("Failed to write node: {}", err);
                return -2
            }
        } else {
            let res = match spawn(&node, "create", node_type) {
                Ok(a) => a,
                Err(err) => {
                    println!("Failed to open editor: {}", err);
                    return -3;
                },
            };

            if !res.success() {
                println!("Editor returned with error code {}", res);
                return -4;
            }

            // if node was not written, there is nothing more to do here
            if !node.node_path().exists() {
                println!("No node was created");
                return -5;
            }

            strip_node_meta(&node, &mut meta);
        }

        let now = time::now().rfc3339().to_string();

        meta.set("created", toml::Value::from(now.clone()));
        meta.set("type", toml::Value::from(node_type.clone()));

        if let Err(err) = meta.save(node.meta_path()) {
            println!("Failed to save node meta file: {}", err);
            fs::remove_file(node.node_path())
                .expect("Failed to removed node file");
            return -7;
        }

        println!("Created Node {}", node.id());
    }

    storage.use_id();
    0
}

pub fn ls(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    let tree = match args.value_of("pattern") {
        Some(p) => match pattern::parse_condition(p) {
            Ok(a) => Some(a),
            Err(err) => {
                println!("Could not parse condition pattern: {}", err);
                return -1;
            },
        }
        None => None
    };

    let num = if args.is_present("num") {
        value_t!(args, "num", usize).unwrap_or_else(|e| e.exit())
    } else {
        storage.config().value().as_ref()
            .and_then(|c| c.find("ls_count"))
            .and_then(|v| v.as_integer()).map(|v| v as usize)
            .unwrap_or(LS_COUNT_DEFAULT)
    };

    let mut lines = value_t!(args, "lines", u64).unwrap_or(1);
    if args.is_present("full") {
        lines = 10000; // TODO, we can do better than this!
    }

    let mut nodes: Vec<nodes::Node> = Vec::new();
    for node in storage.nodes() {
        let mut meta = match node.load_meta() {
            Ok(a) => a,
            Err(e) => {
                println!("Failed to load meta file for node {}: {:?}",
                    node.id(), e);
                continue;
            }
        };

        // check condition
        if let &Some(ref tree) = &tree {
            if !pattern::node_matches(&meta, &tree) {
                continue;
            }
        }

        nodes.push(node);
    }

    nodes.sort_by_key(|v| v.id());
    if !args.is_present("reverse") {
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

pub fn edit(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = nodes::Node::new(storage, id);
    let meta = args.is_present("meta");

    if !node.exists() {
        println!("Node {} does not exist", id);
        return -1;
    }

    if meta {
        return match spawn_meta(&node) {
            Ok(v) => v.code().unwrap_or(-2),
            Err(e) => {
                println!("Failed to spawn editor: {}", e);
                return -3;
            }
        }
    }

    let meta = match node.load_meta() {
        Ok(a) => a,
        Err(e) => {
            println!("Failed to load meta for node {}: {:?}", node.id(), e);
            return -4;
        },
    };

    let nodetype = meta.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("text");
    match spawn(&node, "edit", nodetype) {
        Ok(v) => v.code().unwrap_or(-5),
        Err(e) => {
            println!("Failed to spawn editor: {}", e);
            return -6;
        }
    }
}

pub fn config(config: &nodes::Config, _args: &clap::ArgMatches) -> i32 {
    let mut prog = build_program(&config, "edit", "config");
    prog.push(nodes::Config::config_path().to_string_lossy().into_owned());
    match process::Command::new(&prog[0]).args(prog[1..].iter()).status() {
        Err(e) => {
            println!("Failed to spawn editor: {}", e);
            -1
        }, Ok(s) => s.code().unwrap_or(-2)
    }
}

pub fn rm(storage: &nodes::Storage, args: &clap::ArgMatches) -> i32 {
    let ids = values_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let mut res = 0;
    for id in ids {
        if let Err(e) = nodes::Node::new(storage, id).remove() {
            println!("Failed to remove node {}: {}", id, e);
            res += 1;
        }
    }

    res
}

pub fn ref_path(config: &nodes::Config, args: &clap::ArgMatches) -> i32 {
    let node_ref = args.value_of("ref")
        .expect("No ref argument given, although it is required");
    let NodeRef {id, storage} = match parse_node_ref(node_ref) {
        Some(a) => a,
        None => {
            println!("Invalid node reference: {}", node_ref);
            return -1;
        }, 
    };

    let storage = match storage {
        Some(a) => match config.load_storage(a) {
            Ok(a) => a,
            Err(e) => {
                println!("Failed to load storage {}: {:?}", a, e);
                return -4;
            },
        }, None => {
            let from = match args.value_of("from") {
                Some(a) => a,
                None => {
                    println!("'From' node is required to resolve \
                             'this' storage qualifier");
                    return -2;
                }
            };
            
            match storage_for_path(&config, PathBuf::from(&from)) {
                Some(a) => a,
                None => {
                    println!("Could not get storage for {}", from);
                    return -3;
                },
            }
        },
    };

    let node = nodes::Node::new(&storage, id);
    if !node.exists() {
        println!("Node {} does not exist", node_ref);
        return -5;
    }

    println!("{}", node.node_path().to_string_lossy());
    0
}

pub fn add(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    {
        let node_type = DEFAULT_NODE_TYPE;
        let node = storage.next_node();

        // copy file
        let fname = args.value_of("file").
            expect("No file argument given, although it is required");
        let path = Path::new(fname);
        if let Err(e) = fs::copy(path, node.node_path()) {
            println!("Could not copy file to node {}: {}", fname, e);
            return -1;
        }

        let mut meta = toml::Value::new();
        let now = time::now().rfc3339().to_string();
        meta.set("created", toml::Value::from(now.clone()));
        meta.set("type", toml::Value::from(node_type.clone()));

        meta.save(node.meta_path()).expect("Failed to save node meta file");
        if let Err(err) = meta.save(node.meta_path()) {
            println!("Failed to save node meta file: {}", err);
            fs::remove_file(node.node_path())
                .expect("Failed to removed node file");
            return -2;
        }

        println!("Created Node {}", node.id());
    }

    storage.use_id();
    0
}

// private util
fn program_for_entry(config: &toml::Value, entry: &str) 
        -> Option<Vec<String>> {
    match config.find(&entry) {
        Some(val) => match val {
            &toml::Value::String(ref p) => Some(vec!(p.clone())),
            &toml::Value::Array(ref p) => {
                let mut v: Vec<String> = Vec::new();

                // we only allow strings as arguments
                for val in p {
                    if let Some(arg) = val.as_str() {
                        v.push(arg.to_string());
                    } else {
                        println!("Invalid program arg for entry {}: {}", 
                            entry, val);
                    }
                }

                Some(v)
            }, _ => {
                println!("Invalid program type for entry {}", entry);
                None
            }
        }, None => None
    }
}

fn fallback_program(cat: &str) -> Vec<String> {
    if cat == "create" {
        let editor = match env::var("EDITOR") {
            Ok(a) => a,
            Err(_) => env::var("VISUAL").unwrap_or("vim".to_string()),
        };

        vec!(editor)
    } else {
        vec!("xdg-open".to_string())
    }
}

fn build_program(config: &nodes::Config, cat: &str, ntype: &str)
        -> Vec<String> {
    let config = match config.value() {
        &Some(ref a) => a,
        &None => return fallback_program(cat)
    };

    let mut entry = String::from("programs.");
    entry.push_str(cat);
    entry.push('.');
    entry.push_str(ntype);

    if let Some(prog) = program_for_entry(&config, &entry) {
        return prog;
    }

    entry.clear();
    entry.push_str("programs");
    entry.push_str("defaults");
    entry.push_str(ntype);

    if let Some(prog) = program_for_entry(&config, &entry) {
        return prog;
    }

    entry.clear();
    entry.push_str("programs");
    entry.push_str("defaults");
    entry.push_str("default");

    if let Some(prog) = program_for_entry(&config, &entry) {
        return prog;
    }

    return fallback_program(cat);
}

fn patch_program(node: &nodes::Node, prog: &mut Vec<String>) -> bool {
    lazy_static! {
        static ref REGEX: regex::Regex = 
            regex::Regex::new("\
                (^|[~\\\\])
                @(full_content|\
                    first_line|\
                    id|\
                    node_path|\
                    storage_name|\
                    storage_path|\
                    (meta\\{([^\\}]+)\\}))").expect("Internal regex error");
    }

    // TODO: signal error on return? don't just continue
    // TODO: performance: don't load content multiple times, cache meta?
    
    let mut used = false;
    let mut cpy = String::new();
    'args: for arg in prog.iter_mut() {
        loop {
            cpy.clear();
            cpy.push_str(&arg);
            if let Some(capture) = REGEX.captures(&arg) {
                used = true;
                let first = capture.get(1)
                    .expect("Internal regex capture error");
                cpy.drain(first.start()..first.end());
                match first.as_str() {
                    "full_content" => {
                        let mut s = String::new();
                        let res = File::open(node.node_path())
                            .and_then(|mut f| f.read_to_string(&mut s));
                        if let Err(e) = res {
                            println!("Failed to read '{}': {}", node.id(), e);
                            continue;
                        }

                        cpy.insert_str(first.start(), &s);
                    }, "first_line" => {
                        let f = match File::open(node.node_path()) {
                            Ok(a) => a,
                            Err(e) => {
                                println!("Failed to open '{}': {}", 
                                        node.id(), e);
                                continue;
                            },
                        };

                        let mut reader = BufReader::new(f);
                        let line = match reader.lines().next() {
                            Some(Ok(a)) => a,
                            Some(Err(e)) => {
                                println!("Invalid first line of {}: {}",
                                    node.id(), e);
                                continue;
                            }, None => {
                                println!("Node {} is empty", node.id());
                                continue;
                            },
                        };

                        cpy.insert_str(first.start(), &line);
                    }, "id" => {
                        cpy.insert_str(first.start(), &node.id().to_string());
                    }, "node_path" => {
                        cpy.insert_str(first.start(), 
                            &node.node_path().to_str().unwrap());
                    }, "storage_name" => {
                        cpy.insert_str(first.start(), 
                            node.storage().name());
                    }, "storage_path" => {
                        cpy.insert_str(first.start(), 
                            node.storage().path().to_str().unwrap());
                    }, _ => {
                        if first.as_str().starts_with("meta") {
                            let entry = capture.get(2).unwrap().as_str();
                            let meta = node.load_meta().unwrap();
                            let s = meta.find(entry).and_then(|e|  
                                    toml::ser::to_string_pretty(&e).ok())
                                .unwrap_or("".to_string());
                            cpy.insert_str(first.start(), &s);
                        } else {
                            // Invalid alternative
                            panic!("Unexpected");
                        }
                    }
                }
            } else {
                continue 'args;
            }

            arg.clear();
            arg.push_str(&cpy);
        }
    }

    used
}

fn spawn(node: &nodes::Node, cat: &str, ntype: &str)
        -> io::Result<process::ExitStatus> {
    let config = node.storage().config();
    let mut prog = build_program(&config, cat, ntype);
    let path = node.node_path();

    if !patch_program(&node, &mut prog) {
        prog.push(path.to_str().unwrap().to_string());
    }

    process::Command::new(&prog[0]).args(prog[1..].iter()).status()
}

fn spawn_meta(node: &nodes::Node)
        -> io::Result<process::ExitStatus> {
    let config = node.storage().config();
    let mut prog = build_program(&config, "edit", "meta");
    let path = node.meta_path();

    if !patch_program(&node, &mut prog) {
        prog.push(path.to_str().unwrap().to_string());
    }

    process::Command::new(&prog[0]).args(prog[1..].iter()).status()
}

fn list_node(node: &nodes::Node, lines: u64) {
    let summary = node_summary(&node.node_path(), lines);

    if lines == 1 {
        println!("{}:\t{:<w$}",
            node.id(), summary, w = SUMMARY_SIZE);
    } else {
        println!("{}:", node.id());
        for line in summary.lines() {
            println!("\t{}", line);
        }
        println!();
    }
}


fn node_summary(path: &PathBuf, lines: u64) -> String {
    if lines == 1 {
        short_string(&read_node(&path, lines, false), SUMMARY_SIZE)
    } else {
        read_node(&path, lines, true)
    }
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

    for line in f.lines() {
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

struct NodeRef<'a> {
    id: u64,
    storage: Option<&'a str>
}

fn parse_node_ref<'a>(node_ref: &'a str) -> Option<NodeRef<'a>> {
    lazy_static! {
        static ref REGEX: regex::Regex = 
            regex::Regex::new("([0-9]+)@(?:nodes|n)?:([^@]+)?")
                .expect("Internal invalid regex");
    }

    match REGEX.captures(&node_ref) {
        None => None,
        Some(capture) => {
            let storage = capture.get(2).map(|v| v.as_str());
            let id = capture.get(1)
                .and_then(|v| v.as_str().parse::<u64>().ok());
            let id = match id {
                Some(a) => a,
                None => {
                    println!("Could not parse node ref id");
                    return None;
                },
            };

            Some(NodeRef{id, storage})
        }
    }
}

// TODO: this function should probably check if the storage is
// known (by the given config), and if so set its name correctly

/// Returns the the storage at the given path.
/// Basically tests path and it's parent directory for a 
/// storage file. If it exists (and is valid) returns its name.
/// The name of storage will be set to the folder it is in.
fn storage_for_path(config: &nodes::Config, mut path: PathBuf) 
        -> Option<nodes::Storage> {
    if path.is_relative() {
        path = match env::current_dir() {
            Ok(mut a) => {a.push(path); a},
            Err(_) => {
                println!("Could not retrieve current_dir");
                return None;
            }
        };
    }

    // fn load_storage(config: &nodes::Cofnig, path: PathBuf)
    let load_storage = |path: PathBuf| -> Option<nodes::Storage> {
        let mut cpy = path.clone();
        cpy.pop();
        cpy.pop();
        let name = match cpy.file_name() {
            Some(a) => a.to_str().unwrap(),
            None => return None,
        };

        return nodes::Storage::load(config, name, path).ok();
    };

    if path.is_file() {
        path.pop();
    } 

    if path.is_dir() {
        path.push("storage");
        if path.is_file() {
            path.pop();
            return load_storage(path);
        }
        path.pop();
    }

    path.pop();
    load_storage(path)
}

// TODO: return error/msg
fn parse_meta(s: &str, val: &mut toml::Value) -> bool {
    let parsed = match s.parse::<toml::Value>() {
        Ok(a) => a,
        Err(e) => {
            println!("Failed to parse given meta toml '{}': {:?}", s, e);
            return false;
        }
    };

    // append it
    append_toml(val, &parsed);
    true
}

fn append_toml(dst: &mut toml::Value, src: &toml::Value) {
    match src {
        &toml::Value::Table(ref t) => {
            for pair in t {
                if let Some(val) = dst.find_mut(pair.0) {
                    append_toml(val, pair.1);
                    continue;
                }

                dst.set(pair.0, pair.1.clone());
            }
        }, _ => *dst = src.clone(),
    }
}

fn strip_node_meta(node: &nodes::Node, meta: &mut toml::Value) {
    // TODO: error handling; maybe only try this for textual
    // nodes in the first place? How to differentiate?
    
    // check if we can read the first line, in which case
    // we will check (and strip) it for metadata annotations
    let mut data = Vec::new();

    {
        let file = match File::open(node.node_path()) {
            Ok(a) => a,
            Err(e) => {
                println!("Failed to open created node: {}", e);
                return;
            },
        };

        let reader = BufReader::new(&file);
        let mut lines = reader.lines();
        if let Some(Ok(mut line)) = lines.next() {
            if line.starts_with("nodes: ") {
                line.drain(0..7);
                line = line.replace(";", "\n");
                parse_meta(&line, meta);

                let mut reader = BufReader::new(&file);
                reader.seek(io::SeekFrom::Start(0)).unwrap();
                reader.read_to_end(&mut data).unwrap();
                let idx = data.iter().position(|&v| v == '\n' as u8);
                if let Some(first) = idx {
                    data.drain(0..(first+1));
                }
            }
        } else {
            println!("Could not parse first line");
            return;
        }
    }

    if !data.is_empty() {
        let res = File::create(node.node_path())
            .and_then(|mut f| f.write_all(&data));
        if let Err(err) = res {
            println!("Failed to write stripped node file: {}", err);
            return;
        }
    }
}
