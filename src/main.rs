extern crate nodes;
extern crate toml;
extern crate time;

#[macro_use]
extern crate clap;

use std::path::PathBuf;
use std::fs;
use std::process::Command;

use nodes::State;

// const BASEDIR: &str = "/home/nyorain/.local/share/nodes";
const NODESDIR: &str = "/home/nyorain/.local/share/nodes/nodes";
const METADIR: &str = "/home/nyorain/.local/share/nodes/.meta";

// struct Node {
//     id: u64,
//     ids: String,
//     name: String,
//     tags: Vec<String>,
//     meta: toml::Value
// }
// 
// impl Node {
//     fn from_id(id: u64) -> Node {
//         let mut n = Node {
//             id,
//             ids: id.to_string(),
//             name: String::new(),
//             tags: Vec::new(),
//             meta: toml::Value::Integer(0) // TODO
//         };
//         n
//     }
// 
//     fn meta_path(&self) -> PathBuf {
//         let mut pb = PathBuf::from(METADIR);
//         pb.push(&self.ids);
//         pb
//     }
// 
//     fn node_path(&self) -> PathBuf {
//         let mut pb = PathBuf::from(NODESDIR);
//         pb.push(&self.ids);
//         pb
//     }
// }

// fn command_add(args: &[String]) -> i32 {
//     if args.len() == 0 {
//         println!("Error: add needs a file name");
//         return -1;
//     }
// 
// 
//     let path = Path::new(&args[0]);
//     let name = path.file_stem().expect("Invalid file to add");
// 
//     let mut npathb = PathBuf::from(NODESDIR);
//     npathb.push(name);
//     fs::copy(path, npathb.as_path()).expect("Invalid file to add");
// 
//     0
// }

fn command_rm(args: &clap::ArgMatches) -> i32 {
    let id = value_t!(args, "id", u64).unwrap_or_else(|e| e.exit());
    let ids = id.to_string();

    let mut pathb = PathBuf::from(NODESDIR);
    pathb.push(ids);
    if let Err(e) = fs::remove_file(pathb.as_path()) {
        println!("Could not remove node {}: {}", id, e);
        return -1;
    };

    println!("Removed node {}", id);
    // TODO: remove meta file!

    0
}

fn command_create(args: &clap::ArgMatches) -> i32 {
    let mut state = State::load();
    let id = state.next_id();

    // create node
    let mut pathb = PathBuf::from(NODESDIR);
    pathb.push(id.to_string());
    let path = pathb.as_path();
    let name = path.to_str().expect("Invalid name");
    let mut child = Command::new("nvim").arg(name).spawn().expect("spawn");
    child.wait().expect("wait");

    // if node was not written, there is nothing more to do here
    if !path.exists() {
        println!("No node was created");
        return -1;
    }

    state.use_id();
    let mut pathb = PathBuf::from(METADIR);
    pathb.push("nodes");
    pathb.push(id.to_string());

    let name = match args.value_of("name") {
        Some(name) => name.to_string(),
        None => id.to_string()
    };

    // set meta data
    let mut meta = nodes::toml_new();
    let now = time::now().rfc3339().to_string();
    nodes::toml_set(&mut meta, "name", toml::Value::from(name.clone()));
    nodes::toml_set(&mut meta, "created", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "changed", toml::Value::from(now.clone()));
    nodes::toml_set(&mut meta, "accessed", toml::Value::from(now.clone()));
    nodes::save_toml_file(&meta, pathb);

    // output information
    if args.is_present("name") {
        println!("Created Node {}: {}", id, name);
    } else {
        println!("Created Node {}", id);
    }

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
            (@arg name: !required index(1) "The name, id by default")
        )
        (@subcommand rm =>
            (about: "Removes a node (by id)")
            (@arg id: +required index(1) "The nodes id")
        )
    ).get_matches();

    match matches.subcommand() {
        ("create", Some(s)) => command_create(s),
        ("rm", Some(s)) => command_rm(s),
        _           => panic!("This should not happen"),
    }
}

fn main() {
    std::process::exit(ret_main());
}
