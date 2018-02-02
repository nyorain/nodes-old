#[macro_use] extern crate clap;

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
