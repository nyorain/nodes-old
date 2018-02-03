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
use std::fs::File;
use std::io::prelude::*;

const DEFAULT_NODE_TYPE: &str = "text";
const NAME_SIZE: usize = 20;
const SUMMARY_SIZE: usize = 40;

pub fn create(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    {
        let node_type = args.value_of("type").unwrap_or(DEFAULT_NODE_TYPE);
        let node = storage.next_node();

        if let Some(content) = args.value_of("content") {
            if content.is_empty() {
                println!("No content given");
                return -1;
            }

            let mut f = File::create(node.node_path()).unwrap();
            if let Err(err) = f.write_all(content.as_bytes()) {
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
        }

        let mut meta = toml::Value::new();
        let now = time::now().rfc3339().to_string();

        meta.set("created", toml::Value::from(now.clone()));
        meta.set("type", toml::Value::from(node_type.clone()));

        if let Some(name) = args.value_of("name") {
            meta.set("name", toml::Value::from(name.clone()));
        }

        if let Err(err) = meta.save(node.meta_path()) {
            println!("Failed to save node meta file: {}", err);
            fs::remove_file(node.node_path())
                .expect("Failed to removed node file");
            return -6;
        }

        // output information
        if let Some(name) = args.value_of("name") {
            println!("Created Node {}: {}", node.id(), name);
        } else {
            println!("Created Node {}", node.id());
        }
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

    // debug the built tree
    // pattern::print_cond(&tree);
    // println!();

    let num = value_t!(args, "num", usize).unwrap_or(10);
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
    prog.push(nodes::Config::config_path().to_str().unwrap().to_string());
    match process::Command::new(&prog[0]).args(prog[1..].iter()).status() {
        Err(e) => {
            println!("Failed to spawn editor: {}", e);
            -1
        }, Ok(s) => s.code().unwrap_or(-2)
    }
}

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
        vec!(env::var("EDITOR").unwrap())
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
                    (meta\\{([^\\}]+)\\}))").unwrap();
    }

    // TODO: error handling (some unwraps are or, some are not...)
    // TODO: performance: don't load content multiple times, cache meta
    let mut used = false;
    let mut cpy = String::new();
    'args: for arg in prog.iter_mut() {
        loop {
            cpy.clear();
            cpy.push_str(&arg);
            if let Some(capture) = REGEX.captures(&arg) {
                used = true;
                let first = capture.get(1).unwrap();
                cpy.drain(first.start()..first.end());
                match first.as_str() {
                    "full_content" => {
                        let mut s = String::new();
                        File::open(node.node_path()).unwrap()
                            .read_to_string(&mut s).unwrap();
                        cpy.insert_str(first.start(), &s);
                    }, "first_line" => {
                        let f = File::open(node.node_path()).unwrap();
                        let mut reader = BufReader::new(&f);
                        cpy.insert_str(first.start(), 
                            &reader.lines().next().unwrap().unwrap());
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
                            let s = meta.find(entry)
                                .and_then(|e| { 
                                    toml::ser::to_string_pretty(&e).ok()
                                }).unwrap_or("".to_string());
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
    let name = node.load_meta().unwrap()
        .find("name").and_then(|v| v.as_str())
        .unwrap_or("").to_string();

    if lines == 1 {
        println!("{}:\t{:<w2$}    {:<w3$}",
            node.id(), short_string(&name, NAME_SIZE), summary,
            w2 = NAME_SIZE, w3 = SUMMARY_SIZE);
    } else {
        println!("{}:\t{}", node.id(), name);
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
