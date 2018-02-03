#[macro_use] extern crate clap;
#[macro_use] extern crate lazy_static;
extern crate nodes;
extern crate regex;

mod commands;

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
        (author: "nyorain [at gmail dot com]")
        (about: "Manages your node system from the command line")
        (@arg storage: -s --storage +takes_value "The storage to use")
        (@subcommand create =>
            (about: "Creates a new node")
            (alias: "c")
            (@arg name: !required index(1) "The name, id by default")
            (@arg tags: -t --tag +takes_value !required ... +use_delimiter
                "Tag the node")
            (@arg type: --type +takes_value !required "Node type")
            (@arg content: -c --content +takes_value !required
                "Node content")
        ) (@subcommand rm =>
            (about: "Removes a node (by id)")
            (@arg id: +required +multiple index(1)
                {is_uint}
                "The nodes id. Can also specify multiple nodes")
        ) (@subcommand add =>
            (about: "Creates a new node from an existing file")
            (alias: "a")
            (@arg file: +required index(1) "The file to add")
            (@arg name: !required index(2) "Name of new node, id by default")
        ) (@subcommand ls =>
            (about: "Lists existing notes")
            (@arg pattern: index(1) 
                "Only list nodes matching this pattern")
            (@arg num: -n --num +takes_value
                default_value("10")
                {is_uint}
                "Maximum number of nodes to show")
            (@arg lines: -l --lines +takes_value
                {is_uint}
                "How many lines to show at maximum from a node")
            (@arg full: -f --full conflicts_with("lines") "Print full nodes")
            (@arg sort: -s --sort
                +case_insensitive
                default_value("id")
                +takes_value "Order of displayed nodes")
            (@arg reverse: -R --rev !takes_value !required
                "Reverses the order")
            (@arg reverse_list: -r --revlist !takes_value !required
                "Reverses the display order")
        ) (@subcommand show =>
            (about: "Shows a node")
            (alias: "s")
            (@arg id: +required index(1) {is_uint} "Id of node to show")
            (@arg meta: -m --meta "Shows the meta file instead")
        ) (@subcommand edit =>
            (about: "Edits a node")
            (alias: "e")
            (@arg id: +required index(1) {is_uint} "Id of node to edit")
            (@arg meta: -m --meta "Edit the meta file instead")
        ) (@subcommand config =>
            (about: "Edit config file")
        )
    ).get_matches();

    let config = nodes::Config::load_default().unwrap();
    let mut storage = match matches.value_of("storage") {
        Some(name) => config.load_storage(name).unwrap(),
        None => config.load_default_storage().unwrap(),
    };

    match matches.subcommand() {
        ("rm", Some(s)) => commands::rm(&mut storage, s),
        ("config", Some(s)) => commands::config(&config, s),
        ("edit", Some(s)) => commands::edit(&mut storage, s),
        ("create", Some(s)) => commands::create(&mut storage, s),
        ("ls", Some(s)) => commands::ls(&mut storage, s),
        _ => commands::ls(&mut storage, &clap::ArgMatches::default())
    }
}

fn main() {
    std::process::exit(ret_main());
}
