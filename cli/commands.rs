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
use std::cmp;
use std::process;

use std::io::BufReader;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::io::BufWriter;

use termion::event::Key;
use termion::screen::*;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

const DEFAULT_NODE_TYPE: &str = "text";
const SUMMARY_SIZE: usize = 70; // TODO: dynamic, based on terminal size!
const LS_COUNT_DEFAULT: usize = 30;

pub fn create(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    {
        let node_type = args.value_of("type").unwrap_or(DEFAULT_NODE_TYPE);
        let node = storage.next_node();

        let mut meta = toml::Value::new();

        // tag values
        if args.is_present("tags") {
            let toml_tags = value_t!(args, "tags", toml::Value)
                .unwrap_or_else(|e| e.exit());
            meta.set("tags", toml_tags);
        }

        if let Some(val) = args.value_of("meta") {
            let mut val = val.replace(";", "\n");
            if let Err(err) = parse_meta(&val, &mut meta) {
                println!("Failed to parse 'meta' flag: {}", err);
                return -6;
            }
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

// arg reverse: whether to invert the meaning of the reverse flag
// arg reverse_list: whether to invert the meaning of the reverse_list flag
fn list<'a, 'b>(storage: &'a nodes::Storage, args: &clap::ArgMatches,
        reverse: bool, reverse_list: bool) -> Option<Vec<nodes::Node<'a, 'a>>> {
    let tree = match args.value_of("pattern") {
        Some(p) => match pattern::parse_condition(p) {
            Ok(a) => Some(a),
            Err(err) => {
                println!("Could not parse condition pattern: {}", err);
                return None;
            },
        }
        None => None
    };

    // debug argument
    if args.is_present("debug_condition") {
        if let &Some(ref tree) = &tree {
            pattern::print_cond(tree);
            println!("");
        }
    }

    let archived = args.is_present("archived");
    let num = if args.is_present("num") {
        value_t!(args, "num", usize).unwrap_or_else(|e| e.exit())
    } else {
        storage.config().value().as_ref()
            .and_then(|c| c.find("ls_count"))
            .and_then(|v| v.as_integer()).map(|v| v as usize)
            .unwrap_or(LS_COUNT_DEFAULT)
    };

    let mut nodes: Vec<nodes::Node> = Vec::new();
    let list = if archived { storage.archived() } else { storage.nodes() };
    for node in list {
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
            let meta_node = pattern::MetaNode{
                node: &node,
                meta: &meta,
            };
            if !pattern::node_matches(&meta_node, &tree) {
                continue;
            }
        }

        nodes.push(node);
    }

    nodes.sort_by_key(|v| v.id());
    if reverse ^ !args.is_present("reverse") {
        nodes.reverse();
    }

    nodes.truncate(num);

    if reverse_list ^ !args.is_present("reverse_list") {
        nodes.reverse();
    }

    Some(nodes)
}

pub fn ls(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    let mut lines = value_t!(args, "lines", u64).unwrap_or(1);
    if args.is_present("full") {
        lines = 99999; // TODO, we can do better than this!
    }

    let nodes = match list(storage, args, false, false) {
        Some(n) => n,
        None => {
            return -1;
        }
    };

    for node in nodes {
        list_node(&node, lines);
    }

    0
}

pub fn edit(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    let r: i32;
    let id: u64;
    let idstr = value_t!(args, "id", String).unwrap_or_else(|e| e.exit());

    {
        let node = match storage.parse(&idstr) {
            Err(e) => {
                println!("Invalid node '{}': {}", &idstr, e);
                return -1;
            }, Ok(n) => n,
        };

        id = node.id();
        let meta = args.is_present("meta");
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
        r = match spawn(&node, "edit", nodetype) {
            Err(e) => {
                println!("Failed to spawn editor: {}", e);
                return -5;
            }, Ok(v) => match v.code() {
                Some(v2) => v2,
                None => {
                    println!("Warning: Signal termination detected");
                    -6
                }
            }
        };
    }

    storage.edited(id);
    r
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

// helper function that applies the given function on all the ids
// passed in the given argument (if present) or otherwise over stdin.
// If operating on stdin, returns the number of invalid lines, otherwise 0.
// The passed function will be called for every node and must return
// whether it was succesful.
pub fn operate_ids_stdin<F: Fn(&mut nodes::Node) -> bool>(
        storage: &nodes::Storage, args: &clap::ArgMatches,
        argname: &str, op: F) -> i32 {

    let mut res = 0;
    if args.is_present(argname) {
        let ids = values_t!(args, argname, String).unwrap_or_else(|e| e.exit());
        for idstr in ids {
            let mut node = match storage.parse(&idstr) {
                Err(e) => {
                    println!("Invalid node '{}': {}", &idstr, e);
                    res += 1;
                    continue;
                }, Ok(n) => n,
            };

            if !op(&mut node) {
                res += 1
            }
        }
        res
    } else {
        let stdin = io::stdin();
        for rline in stdin.lock().lines() {
            let line = match rline {
                Err(err) => {
                    println!("Failed to read line: {}", err);
                    res += 1;
                    continue
                }, Ok(l) => l,
            };

            let mut node = match storage.parse(&line) {
                Err(e) => {
                    println!("Invalid node '{}': {}", line, e);
                    res += 1;
                    continue;
                }, Ok(n) => n,
            };

            if !op(&mut node) {
                res += 1
            }
        }

        res
    }
}

pub fn rm(storage: &nodes::Storage, args: &clap::ArgMatches) -> i32 {
    operate_ids_stdin(storage, args, "id", |node: &mut nodes::Node| -> bool {
        if let Err(e) = node.remove() {
            println!("Failed to remove node {}: {}", node.id(), e);
            return false
        }
        true
    })
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
                .expect("Failed to remove node file");
            return -2;
        }

        println!("Created Node {}", node.id());
    }

    storage.use_id();
    0
}

pub fn archive(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    operate_ids_stdin(storage, args, "id", |node: &mut nodes::Node| -> bool {
        if let Err(e) = node.toggle_archive() {
            println!("Failed to (un)archive node {}: {}", node.id(), e);
            return false;
        }
        true
    })
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

fn fallback_program(cat: &str, ntype: &str) -> Vec<String> {
    if cat == "create" { // switch over ntype?
        let editor = match env::var("EDITOR") {
            Ok(a) => a,
            Err(_) => env::var("VISUAL").unwrap_or("vim".to_string()),
        };

        vec!(editor)
    } else if cat == "show" && ntype == "text" {
        // TODO
        vec!("less".to_string())
    } else {
        vec!("xdg-open".to_string())
    }
}

fn build_program(config: &nodes::Config, cat: &str, ntype: &str)
        -> Vec<String> {
    let config = match config.value() {
        &Some(ref a) => a,
        &None => return fallback_program(cat, ntype)
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

    return fallback_program(cat, ntype);
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
    // TODO: use terminal width
    let summary = node_summary(&node.node_path(), lines, SUMMARY_SIZE);

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


fn node_summary(path: &PathBuf, lines: u64, width: usize) -> String {
    if lines == 1 {
        short_string(&read_node(&path, lines, false), width)
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

// Parses the given string as meta tags (toml) and tries to add it
// to the already existing meta toml table.
fn parse_meta(s: &str, val: &mut toml::Value) -> Result<(), String> {
    let parsed = match s.parse::<toml::Value>() {
        Ok(a) => a,
        Err(e) => {
            return Err(format!("Failed to parse meta toml '{}': {:?}", s, e));
        }
    };

    // append it
    return append_toml(val, &parsed);
}

// Tries to appends the toml stored in src to the toml stored in dst.
// Will fail if there are duplicates, but append to arrays and insert
// into tables (recursively).
fn append_toml(dst: &mut toml::Value, src: &toml::Value) -> Result<(), String> {
    // we only handle top-level entries, everything else is handled
    // recursively
    match src {
        // table: recursive call but also insert new entries
        &toml::Value::Table(ref table) => {
            for pair in table {
                if let Some(val) = dst.find_mut(pair.0) {
                    match append_toml(val, pair.1) {
                        Err(err) => return Err(err),
                        Ok(_) => continue,
                    }
                }

                // new entry, insert it
                dst.set(pair.0, pair.1.clone());
            }
        },
        // array: insert new values at end but make sure that the types
        // still match
        &toml::Value::Array(_) => {
            let dst_array = dst.as_array_mut().unwrap();
            if let &toml::Value::Array(ref src_array) = src {
                for srcel in src_array {
                    if !dst_array.is_empty() && !dst_array[0].same_type(srcel) {
                        return Err("Incomptabile meta array value types"
                            .to_string())
                    }

                    dst_array.push(srcel.clone());
                }
            } else {
                return Err("Incomptabile meta array value".to_string())
            }
        },
        // otherwise we try to overwrite a value. This is an error.
        // mainly required when called recursively
        _ => {
            // *dst = src.clone()
            return Err("Duplicate meta value".to_string())
        }
    }

    Ok(())
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

        // TODO: read all lines until one without "nodes: " comes?
        let reader = BufReader::new(&file);
        let mut lines = reader.lines();
        if let Some(Ok(mut line)) = lines.next() {
            if line.starts_with("nodes: ") {
                line.drain(0..7);
                line = line.replace(";", "\n");
                if let Err(err) = parse_meta(&line, meta) {
                    println!("Invalid node meta: {}", err);
                    return
                }

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

struct SelectNode<'a, 'b: 'a> {
    node: nodes::Node<'a, 'b>,
    summary: String,
    selected: bool,
}

fn write_select_list<W: Write>(screen: &mut W, nodes: &Vec<SelectNode>,
        start: usize, current: usize, starty: u16, maxx: u16, maxy: u16) {
    let x = 1;
    let mut y = starty;
    let mut i = start;
    for node in nodes[start..].iter() {
        if y > maxy {
            break;
        }

        if i == current {
            write!(screen, "{}",
                termion::color::Bg(termion::color::LightGreen)).unwrap();
        }

        if node.selected {
            write!(screen, "{}",
                termion::color::Fg(termion::color::LightRed)).unwrap();
        }

        let idstr = node.node.id().to_string();
        write!(screen, "{}{}: {:<w$}{}{}",
            termion::cursor::Goto(x, y),
            idstr, node.summary,
            termion::color::Bg(termion::color::Reset),
            termion::color::Fg(termion::color::Reset),
            w = (maxx as usize) - idstr.len() - 2).unwrap();

        y += 1;
        i += 1;
    }
}

// NOTE: experimental!
pub fn select(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    // problem: when stdin isn't /dev/tty
    // let tty = fs::File::open("/dev/tty").unwrap();
    // TODO: https://github.com/redox-os/termion/blob/master/src/sys/unix/size.rs
    let (maxx, maxy) = match termion::terminal_size() {
        Ok((x,y)) => (x,y),
        _ => (80, 100) // guess
    };

    let lnodes = match list(storage, args, false, true) {
        Some(n) => n,
        None => {
            return -1;
        }
    };

    let mut nodes: Vec<SelectNode> = Vec::new();
    for node in lnodes {
        let summary = node_summary(&node.node_path(), 1, maxx as usize);
        nodes.push(SelectNode{
            node: node,
            summary: summary,
            selected: false
        });
    }

    // setup terminal
    {
        let mut start: usize = 0; // start index in node vec
        let mut current: usize = 0; // current/focused index in node vec
        let mut currenty: u16 = 1; // current/focused y position
        let mut gpending = 2;

        let stdin = io::stdin();
        let raw = match termion::get_tty().and_then(|tty| tty.into_raw_mode()) {
            Ok(r) => r,
            Err(err) => {
                println!("Failed to transform tty into raw mode: {}", err);
                return -2;
            }
        };

        let ascreen = AlternateScreen::from(raw);
        let mut screen = BufWriter::new(ascreen);
        if let Err(err) = write!(screen, "{}", termion::cursor::Hide) {
            println!("Failed to hide cursor in selection screen: {}", err);
            return -3;
        }

        write_select_list(&mut screen, &nodes, start, current, 1, maxx, maxy);
        screen.flush().unwrap();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('q') => {
                    break;
                }
                Key::Char('j') if current < nodes.len() - 1 => {
                    current += 1;
                    if currenty == maxy {
                        start += 1;
                    } else {
                        currenty += 1;
                    }
                },
                Key::Char('G') => {
                    current = nodes.len() - 1;
                    currenty = cmp::min(nodes.len() - 1, maxy as usize) as u16;
                    start = cmp::max((current as i32) - (maxy as i32), 0) as usize;
                },
                Key::Char('g') => {
                    if gpending == 1 {
                        start = 0;
                        current = 0;
                        currenty = 1;

                        gpending = 2;
                    } else {
                        gpending = 0;
                    }
                },
                Key::Char('k') if current > 0 => {
                    current -= 1;
                    if currenty == 1 {
                        start -= 1;
                    } else {
                        currenty -= 1;
                    }
                },
                Key::Char('\n') => {
                    nodes[current].selected ^= true;
                },
                Key::Char('e') => { // edit
                    // TODO: notetype
                    // maybe write common function (shared with edit)?
                    match spawn(&nodes[current].node, "edit", "text") {
                        Err(e) => {
                            eprintln!("Failed to spawn editor: {}", e);
                        }, Ok(v) => if v.code().is_none() {
                            println!("Warning: Signal termination detected");
                        }
                    }

                    // TODO: refresh summary for this node

                    // TODO: fighting the borrow checker
                    // storage.edited(nodes[current].node.id());
                    write!(screen, "{}", termion::clear::All).unwrap();
                },
                Key::Char('s') => { // show
                    // TODO
                    match spawn(&nodes[current].node, "show", "text") {
                        Err(e) => {
                            eprintln!("Failed to spawn program: {}", e);
                        }, Ok(v) => if v.code().is_none() {
                            println!("Warning: Signal termination detected");
                        }
                    }

                    // same TODO s as in edit
                    write!(screen, "{}", termion::clear::All).unwrap();
                },
                // TODO:
                // - use numbers for navigation
                // - a: archive
                // - r: remove (with confirmation?)
                // - somehow show tags/some meta field (already in preview?)
                //   should be configurable
                //   additionally? edit/show meta file
                // - should a/r be applied to all selected? or to the currently
                //   hovered? maybe like in ncmpcpp? (selected? selected : hovered)
                // - allow to open/show multiple at once?
                //   maybe allow to edit/show selected?
                // - less-like status bar or something?
                _ => (),
            }

            if gpending < 2 {
                gpending += 1;
            }

            // TODO: only render changed lines?
            write_select_list(&mut screen, &nodes, start, current, 1, maxx, maxy);
            screen.flush().unwrap();
        }

        write!(screen, "{}", termion::cursor::Show).unwrap();
    }

    for node in nodes {
        if node.selected {
            println!("{}", node.node.id());
        }
    }

    0
}

pub fn show(storage: &mut nodes::Storage, args: &clap::ArgMatches) -> i32 {
    // TODO: use spawn and allow different programs as well
    // requires correct types detection first
    // don't use "cat" or something for text node types (unless manually
    // specified), just print out the lines by default (internally!)
    // XXX: compare to how show in selected is currently implemented
    let idstr = value_t!(args, "id", String).unwrap_or_else(|e| e.exit());
    let node = match storage.parse(&idstr) {
        Err(e) => {
            println!("Invalid node '{}': {}", &idstr, e);
            return -1;
        }, Ok(n) => n,
    };

    let meta = args.is_present("meta");
    let path = if meta { node.meta_path() } else { node.node_path() };

    let mut s = String::new();
    let res = File::open(path).and_then(|mut f| f.read_to_string(&mut s));
    if let Err(e) = res {
        println!("Failed to read '{}': {}", node.id(), e);
        return -2;
    }

    print!("{}", s);
    0
}
