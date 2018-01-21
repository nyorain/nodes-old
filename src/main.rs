extern crate toml;
extern crate time;
extern crate nodes;

#[macro_use]
extern crate clap;

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
            (@arg tags: -t --tag +takes_value !required ... +use_delimiter
                "Tag the node")
            (@arg type: --type +takes_value !required "Node type")
            (@arg content: -c --content +takes_value !required 
                "Node content")
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
                possible_values(&nodes::NodeSort::variants())
                +takes_value "Order of displayed nodes")
            (@arg reverse: -r --reverse !takes_value !required
                "Reverses the order")
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
        ) (@subcommand estate =>
            (about: "Edit state file (development)")
        )
    ).get_matches();

    match matches.subcommand() {
        ("create", Some(s)) => nodes::command_create(s),
        ("rm", Some(s)) => nodes::command_rm(s),
        ("add", Some(s)) => nodes::command_add(s),
        ("ls", Some(s)) => nodes::command_ls(s),
        ("edit", Some(s)) => nodes::command_edit(s),
        ("show", Some(s)) => nodes::command_show(s),
        ("estate", Some(s)) => nodes::command_open_state(s),
        _           => panic!("This should not happen"),
    }
}

fn main() {
    std::process::exit(ret_main());
}
