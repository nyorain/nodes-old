extern crate nodes;
extern crate toml;
extern crate time;

#[macro_use]
extern crate clap;

use std::path::PathBuf;
use std::path::Path;
use std::fs;
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
        None => node.id.to_string()
    };

    // set meta data
    let mut meta = nodes::toml_new();
    let now = time::now().rfc3339().to_string();
    nodes::toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    nodes::toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));
    nodes::save_toml_file(&meta, node.meta_path());

    // output information
    if args.is_present("name") {
        println!("Created Node {}: {}", node.id, name);
    } else {
        println!("Created Node {}", node.id);
    }

    0
}

fn command_ls(_args: &clap::ArgMatches) -> i32 {
    let mut ids: Vec<_> = fs::read_dir(NODESDIR).unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| !p.is_dir())
        .map(|p| -> u64 {p.file_stem().unwrap()
            .to_str().unwrap()
            .parse().unwrap()
        }).collect();

    ids.sort();
    for id in ids {
        let node = Node {id};
        let meta = nodes::parse_toml_file(node.meta_path());
        let name = nodes::toml_get(&meta, "name").unwrap().as_str().unwrap();
        println!("{}: \t{}", id, name);
    }

    0
}

fn command_open(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let node = Node {id};

    let path = node.node_path();
    if !path.exists() {
        println!("Node {} does not exist", id);
        return -1;
    }

    let pname = path.to_str().expect("Invalid name");
    let mut child = Command::new("nvim").arg(pname).spawn().expect("spawn");
    child.wait().expect("wait");

    // TODO: modify state: last accessed/last changed

    0
}

fn ret_main() -> i32 {
    let matches = clap_app!(nodes =>
        (version: "0.1")
        (setting: clap::AppSettings::VersionlessSubcommands)
        (setting: clap::AppSettings::SubcommandRequired)
        (author: "nyorain [at gmail dot com]")
        (about: "Manages local nodes")
        (@subcommand create =>
            (about: "Creates a new node")
            (alias: "c")
            (@arg name: !required index(1) "The name, id by default")
        ) (@subcommand rm =>
            (about: "Removes a node (by id)")
            (@arg id: +required index(1) "The nodes id")
        ) (@subcommand add =>
            (about: "Creates a new node from an existing file")
            (alias: "a")
            (@arg file: +required index(1) "The file to add")
            (@arg name: !required index(2) "Name of new node, id by default")
        ) (@subcommand ls =>
            (about: "Lists all existing notes (id: name)")
        ) (@subcommand open =>
            (about: "Opens a node")
            (alias: "o")
            (@arg id: +required index(1) "Id of node to open")
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
