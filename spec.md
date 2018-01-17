# NODES

directories:

baseDir: base directory. Default on unix: ~/.local/share/nodes
nodesDir: stores all nodes. Default: $baseDir/nodes
metaDir: stores all metadata. Default $baseDir/.meta
cacheDir: temporary caches for faster processing. Default $baseDir/.cache
configDir: stores user-accesible config files and plugins.
  Default on unix: ~/.config/nodes

$nodesDir holds all nodes files organized in some way.
The default is unordered, e.g. all nodes are top-level files.

$metaDir can be used by all plugins.
It will e.g. be used for node connections, tags, colors.
Also holds internal config values and information.

$cacheDir can be used by plugins for temporary data.
Deleting $cacheDir should be possible at any time, with the only
impact being worse performance (for the next usage[s]).
It will usally be used to store shared cache data like additional
tag lookup tables.

# A node

A node is a piece of information.
Nothing else of its shape is specified, its definition and usage
is mainly up to the user.

There are a few things every node has:

- An id: unique for every node (even if deleted).
  By default this will be just the number of the node.
  Cannot be changed over the lifetime of the node.
- A name: node representation to the user.
  Its id by default, can be changed at any time.
  Not unique in any way.
- Metadata: which metadata is stored depends on the configuration.
  Plugins can add own metadata.
  All metadata can be changed. Examples/possibilities are:
	- time of creation
	- type/filetype
	- tags
	- color
	- incoming/outgoing links
	- history
	- status (like archived, trashed, additional meta-tags etc)
	- summary (short description, content)
  Which of these meta information belong to a plugin?

# Default implementation

The default implementation allows to deal with nodes from the command line.
It has no graphical user interface but can be connected to external programs
like an editor, browser or image/video/audio programs.

### Commands

Commands that have a shortcut can be used with the shortcut.
For example `nodes a example.png` will add the example.png file
as node.

`add [options] <file> [<name>]`

shortcut: 'a'

Adds the given file as a new node.
By default, the name of the node will be the basename of
the given file, but it can be overwritten with <name>.
When <name> is id, the nodes id will be used as name.
Will print out the name and id of the added node.

options:
	-l --link:					Hardlink the file
	-s --symlink				Symlink the file
	-t --tags		[tags]		Add the given tags

---

`create [options] [<name>] [<string>]`

shortcut 'c'

Create a new node with the given name.
If a string is given, it will be written into the created node,
otherwise an editor will be opened.
Will print out name and id of the created node.

options:
	-n --name		[name]		Specify the nodes name
	-f --file		[type]		Specify the nodes file type
	-t --tags		[tags]		Add the given tags

---

`show [options] <id>`
`show [options] <name>`

shortcut 's'

Show the contents of node <id> or <name>.
If there are multiple nodes with given <name>, will show
a list of them.
If it is a text-like node it will simply be output, otherwise
opened with the associated program.

options:
	TODO: something about the filetype

---

`rm [options] <id>`
`rm [options] <name>`

Deletes the node <id> or <name>.
Only works for <name> if it is unique.
Will delete it/move it to the trash according to configuation

options:
	TODO: Something about trash/delete

---

`last [<type>]`

shortcut: 'l'

Will output the id and name of the last accessed node.
If no type is given, will output the last changed node.
Otherwise type can be:

	- changed (for content-wise changes) [default]
	- created
	- viewed
	- meta (for meta changes)

---

`ls [<pattern>]`

Lists all nodes that match the given search pattern.
If <pattern> is empty lists all current nodes.
<pattern> can contain tags, name, content or
metadata to search for.

TODO: pattern format

---

`config [<type>]`

Open up the config in an editor.
Will be the default config if no type is given.
