# Default implementation [ideas]

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
