extern crate nodes;
extern crate toml;
extern crate time;

#[macro_use]
extern crate clap;

use std::path::PathBuf;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;

use nodes::State;

const NODESDIR: &str = "/home/nyorain/.local/share/nodes/nodes";
const METADIR: &str = "/home/nyorain/.local/share/nodes/.meta";

struct Node {
    id: u64,
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

fn command_add(args: &clap::ArgMatches) -> i32 {
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

    let mut meta = nodes::toml_new();
    let now = time::now().rfc3339().to_string();
    nodes::toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    nodes::toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));
    nodes::save_toml_file(&meta, node.meta_path());

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

fn command_rm(args: &clap::ArgMatches) -> i32 {
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

fn command_create(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let node = Node {id: state.next_id()};

    // create node
    let path = node.node_path();
    let pname = path.to_str().expect("Invalid name");
    let mut child = Command::new("nvim").arg(pname).spawn().expect("spawn");
    child.wait().expect("wait");

    // if node was not written, there is nothing more to do here
    if !path.exists() {
        println!("No node was created");
        return -1;
    }

    state.use_id();
    let name = match args.value_of("name") {
        Some(name) => name.to_string(),
        None => "".to_string()
    };

    // set meta data
    let mut meta = nodes::toml_new();
    let now = time::now().rfc3339().to_string();
    nodes::toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    nodes::toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));

    if let Some(tags) = args.values_of("tags") {
        let mut collected: Vec<toml::Value> = Vec::new();
        for tag in tags {
            collected.append(&mut tag.split_whitespace()
                .map(|x| toml::Value::String(x.to_string()))
                .collect());
        }
        nodes::toml_set(&mut meta, "tags", toml::Value::Array(collected));
    }

    nodes::save_toml_file(&meta, node.meta_path());

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

fn command_ls(args: &clap::ArgMatches) -> i32 {
    let name = args.value_of("name");
    let tag = args.value_of("tag");
    let sort = value_t!(args, "sort", NodeSort).unwrap_or_else(|e| e.exit());
    let mut num = value_t!(args, "num", u64).unwrap_or_else(|e| e.exit());
    let mut nodes: Vec<FoundNode> = Vec::new();

    for id in nodes_list() {
        let node = Node {id};
        let meta = nodes::parse_toml_file(node.meta_path());
        let nname = nodes::toml_get(&meta, "name").unwrap().as_str().unwrap();

        // check if node has name
        if let Some(name) = name {
            if name != nname {
                continue;
            }
        }

        // check for tag
        if let Some(tag) = tag {
            let mut found = false;
            let ntags = nodes::toml_get(&meta, "tags")
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
            nodes::toml_get(&meta, "accessed").unwrap().
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

fn command_open(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = Node {id};

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

    let pname = path.to_str().expect("Invalid name");
    let mut child = Command::new("nvim").arg(pname).spawn().expect("spawn");
    child.wait().expect("wait");

    if !meta {
        let mut state = State::load();
        state.set("last.accessed", toml::Value::Integer(id as i64));

        let mut meta = nodes::parse_toml_file(node.meta_path());
        let now = time::now().rfc3339().to_string();
        nodes::toml_set(&mut meta, "accessed", toml::Value::from(now));
        nodes::save_toml_file(&meta, node.meta_path());
    }

    0
}

// in which way to sort a node list
arg_enum!{
    #[derive(PartialEq, Debug)]
    enum NodeSort {
        ID, // by node id
        LA, // last accessed
        Name
    }
}

fn ret_main() -> i32 {
    fn is_uint(v: String) -> Result<(), String> {
        if let Err(_) = v.parse::<u64>() {
            Err(format!("Could not parse '{}' as unsigned number", v))
        } else {
            Ok(())
        }
    }

    let matches = clap_app!(nodes =>
        (version: "0.1")
        (setting: clap::AppSettings::VersionlessSubcommands)
        (setting: clap::AppSettings::SubcommandRequired)
        (author: "nyorain [at gmail dot com]")
        (about: "Manages your node system from the command line")
        (@subcommand create =>
            (about: "Creates a new node")
            (alias: "c")
            (@arg name: !required index(1) "The name, id by default")
            (@arg tags: -t --tags +takes_value !required ... +use_delimiter
                "Tags for the node to create")
        ) (@subcommand rm =>
            (about: "Removes a node (by id)")
            (@arg id: +required index(1) {is_uint} "The nodes id")
        ) (@subcommand add =>
            (about: "Creates a new node from an existing file")
            (alias: "a")
            (@arg file: +required index(1) "The file to add")
            (@arg name: !required index(2) "Name of new node, id by default")
        ) (@subcommand ls =>
            (about: "Lists existing notes")
            (@arg name: index(1) "Only list nodes with this name")
            (@arg tag: -t --tag +takes_value "Only list nodes with this tag")
            (@arg num: -n --num +takes_value
                default_value("10")
                {is_uint}
                "Maximum number of nodes to show")
            (@arg sort: -s --sort
                +case_insensitive
                default_value("id")
                possible_values(&NodeSort::variants())
                +takes_value "Order of displayed nodes")
            (@arg reverse: -r --reverse !takes_value !required
                "Reverses the order")
        ) (@subcommand open =>
            (about: "Opens a node")
            (alias: "o")
            (@arg id: +required index(1) {is_uint} "Id of node to open")
            (@arg meta: -m --meta "Open the meta file instead")
        )
    ).get_matches();

    match matches.subcommand() {
        ("create", Some(s)) => command_create(s),
        ("rm", Some(s)) => command_rm(s),
        ("add", Some(s)) => command_add(s),
        ("ls", Some(s)) => command_ls(s),
        ("open", Some(s)) => command_open(s),
        _           => panic!("This should not happen"),
    }
}

fn main() {
    std::process::exit(ret_main());
}
