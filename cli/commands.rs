extern crate nodes;
extern crate time;

use super::clap;
use self::nodes::toml;
use self::nodes::toml::ValueImpl;

use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

pub fn create(args: &clap::ArgMatches) -> i32 {
    let config = nodes::Config::load_default().unwrap();
    let mut storage = config.load_default_storage().unwrap();

    {
        let node = storage.next_node();
        let npath = node.node_path();

        if let Some(content) = args.value_of("content") {
            let mut f = File::create(npath).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        } else {
            let pname = npath.to_str().unwrap();
            Command::new("nvim").arg(pname)
                .spawn().expect("spawn")
                .wait().expect("wait");

            // if node was not written, there is nothing more to do here
            if !npath.exists() {
                println!("No node was created");
                return -1;
            }
        }

        let name = match args.value_of("name") {
            Some(name) => name.to_string(),
            None => "".to_string()
        };

        let mut meta = toml::Value::new();
        let now = time::now().rfc3339().to_string();

        meta.set("name", toml::Value::from(name.clone()));
        meta.set("created", toml::Value::from(now.clone()));
        meta.set("changed", toml::Value::from(now.clone()));
        meta.set("accessed", toml::Value::from(now.clone()));

        meta.save(node.meta_path()).expect("Failed to save node meta file");

        // output information
        if args.is_present("name") {
            println!("Created Node {}: {}", node.id(), name);
        } else {
            println!("Created Node {}", node.id());
        }
    }

    storage.use_id();

    0
}
