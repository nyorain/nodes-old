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

$metaDir can be used by everyone to store metadata either specific
to single nodes or for a node program.
It will e.g. be used for node connections, tags, colors.
Also holds internal config values and information.

$cacheDir can be used by plugins for temporary data.
Deleting $cacheDir should be possible at any time, with the only
impact being worse performance (for the next usage(s)).
It will usally be used to store shared cache data like additional
tag lookup tables.

# A node

A node is a piece of information.
Nothing else of its shape is specified, its definition and usage
is mainly up to the user.

There are a few things every node has:

- An id: unique for every node (even after deletion).
  Internally just uses a counter that is increased with every new node.
  Cannot be changed over the lifetime of the node.
- A name: node representation to the user.
  Default unspecified, up to program and config (may be empty or id).
  Not unique in any way.
- Metadata: which metadata is stored depends on the configuration.
  Everyone can add own metadata.
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
