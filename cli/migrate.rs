extern crate chrono;
use chrono::DateTime;
use chrono::Utc;
use std::io::prelude::*;
use std::fs::File;

fn output<'a, 'b>(storage: &'a nodes::Storage, archived: bool) {
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

        let mut content = String::new();
        let path = node.node_path();
        File::open(path).and_then(|mut f| f.read_to_string(&mut content)).unwrap();
        content = content.replace("'", "''"); // escape for sql

        let created = meta.get("created").unwrap().as_str().unwrap();
        let created = created.parse::<DateTime<Utc>>().unwrap();
        let created = created.format("%Y-%m-%d %H:%M:%S").to_string();

        println!("INSERT INTO nodes(id, content, created, edited, viewed, archived)
    VALUES ({}, '{}', datetime('{}'), datetime('{}'), datetime('{}'), {});",
        node.id(), content, created, created, created, archived);

        // tags
        let tags = meta.get("tags");
        if tags.is_none() {
            continue;
        }

        let tags = tags.unwrap().as_array();
        if tags.is_none() {
            eprintln!("Non-array tags on node {}", node.id());
            continue;
        }

        for tag in tags.unwrap() {
            let stag = tag.as_str();
            if stag.is_none() {
                eprintln!("Non-array tag on node {}: {}", node.id(), tag);
                continue;
            }

            let tag = stag.unwrap().replace("'", "''");
            println!("INSERT INTO tags(node, tag) VALUES ({}, '{}');",
                node.id(), tag);
        }
    }
}

fn main() {
    let config = nodes::Config::load_default().expect("Error loading config");
    let storage = config.load_default_storage().unwrap();
    output(&storage, false);
    output(&storage, true);
}
