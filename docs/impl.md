# Default implementation [ideas]

The default implementation allows to deal with nodes from the command line.
It has no graphical user interface but can be connected to external programs
like an editor, browser or image/video/audio programs.

The notes below are partly outdated and were just used as first reference
or sandbox before implementing something.

## Node id languge

So nodes in one storage can be referred to uniquely by an id.
But there should a syntax that makes it clear the node is meant,
and not just a number (to e.g. allow editor extensions to follow
the link). And one should also be able to reference nodes in
another storage.

Ideas:

```
nodes@42 # reference node 42 in this/default storage
@42  # same, but as shortcut. Optionally supported since it might
     # also be used in different ways

nodes:public@42 # refer to node 42 in storage public
public@42  # same, shortcut
```

This might actually be not so intuitive since we don't refer
to 'something @ 42', and rather to 'node 42 @ some storage',
or 'node 42 in the nodes systems'.
So maybe rather use something like this:

```
42@nodes
42@

42@public
42@public.nodes
42@public:nodes
42@nodes.public
42@nodes:public
```

Having to write @nodes everytime refering to a node sucks.
And both shortcuts '42@' and '@42' as well as '42@public' for
explicit-storage references might look to close to an email/are
too short to be a good reference.

```
nodes:42
n:42
nodes://42
nodes://42@public
nodes://public.42

nodes.42
nodes.public.42
```

The name nodes is probably to general to be established as a reference
name like this. Maybe rename the project while we can?
nodes + thoughts = thoudes?
fragments + nodes = frodes?
note + thought = thote?
yooi, like simple/plain?

... naaah

So the at sign is probably the best idea.
We could use '42@nodes/public' as the full form.
It is distuingusable from an email address (by the last '/'), not too
short and the meaning should be obvious.
We can then introduce shorthands like '42@nodes/' for using the
default/this storage, and also '42@/public', simply not using
'nodes'. Combining both shorthands would produce '42@/', meaning
the node 42 in this/default storage.

Idea: distinguis this/default storage. As in paths, this could
be signaled by '.' and the default storage as the root.
'753@.' 753 in this storage (maybe allow '753@' as further shortened ref?)
  Would be '753@nodes.' explicitly.
  But the '.' could be confusing and making it more like an email.
  Rather use ':'?
'234@/' 234 in the default storage
  Or maybe don't even allow to refer to the default storage?
  It could be changed after all.

# Final result #1

```
32@nodes/project 	# node 32 in storage 'project'
92@nodes/			# node 92 in the same storage
429@/				# shortcut for node 429 in same storage
253@/public			# shortcut for node 253 in storage 'public'
```

Or use ':' instead of '/'?

The same storage obviously only makes sense when used in a node.
If used somewhere else, it's up to the tool/processor how to handle it.
Possible alternative are: 
  - just ignore it, don't parse it as nodes reference
  - parse it as nodes reference but produce an error
  - use the default storage

### Syntax

REF ::= id@(nodes|n)?:(storage)?
id ::= <unsigned number>
storage ::= <identifier, [^@]+>

Parsers may remove the ? behin (nodes|n) for a more conservative
matching. Regex (rust syntax):

```
([0-9]+)@(?:nodes|n)?:([^@]+)?
```

- The first match group is the id number.
- The second match group is the storage qualifier.
  Empty means the 'this' storage.

## Node information language

Really simple language allowing to associate information with nodes.
Something like `name="Some name";tags+"some tag";tags+"another";`?
Allows to set own metadata identifiers.

Simple start: 
  - if the first line a node starts with '[nodes]', will parse the
    rest of line
	- also allow '# [nodes]' or '// [nodes]'?
	- in this case we could also extend it to mulitple lines
	-   --> later on, not now
  - also strips the first line from the node
  - all occurences of ';' are replaced with a newline
  - will parse the result as plain toml
  	- introduce things like that '+' syntax later on if needed

This means a node like this:

```
[nodes] name='some random node name'; tags=['tag1', 'tag2']; color='red'
This here is a random node.
It will have the name and tags and additional metadata as specified above.
Those will override values specified in command line.
```

Some ideas/changes:

  - parse all lines from the beginning that have the tag
  - [nodes] as tag might suck a bit, let's rather choose "nodes:"
    to also keep it a bit like the vim in-file settings syntax
	- not so sure about this
  - if there is a newline after the last 'nodes' line, it will also
    be removed from the real node

```
nodes: name = 'some name'
nodes: tags = ['tag1', 'tag2']

Some node
```

## Library design

Config:
  - general data, loaded config file
  - can be used to get storages (or parse node references)

Storage:
  - One specific node storage
  - name and path
  - loaded state file
  - can be used to receive nodes by id
  - or otherwise operate on nodes (ls, add, rm etc)

Node:
  - Information about one node, functionality on that node (cached?)
  - (only cached, loaded if needed?) meta file

Example program that deletes (if existent) the node 42 from all storages:

```
let config = node::Config::load(); // loads config from default path
for storage in config.load_storages() {
	// #1: using storage rm functionality
	match storage.remove_by_id(42) {
		Ok(_) => println!("Node 42 removed from {}", storage),
		Err(err) => println!("Removing failed from {}: {}", storage, err);
	}

	// #2: explicity alternative with more error handling
	let node = match storage.node_by_id(42) {
		Ok(a) => a,
		None => {
			println!("Storage {} has no node 42", storage);
			continue;
		},
	}

	match node.rm() {
		Ok(_) => println!("Node 42 removed from {}", storage),
		Err(err) => println!("Failed to remove 42 from {}: {}, storage, err),
	}
}
```

### NextNode idea

```
pub struct NextNode<'a> {
    node: Node,
    id_borrow: &'a mut u64 // mut to make it unique borrowed
}

impl NextNode {
    pub fn node(&self) -> &Node {
        self.node
    }

    pub fn create(self) -> Node {
        self.id_borrow += 1;
        self.node
    }
}

```

## Patterns the second

Start with simple logic.

```
has("color") // node has meta entry "color"
has_string("color") // node has meta entry "color" with type string
has_array("color) // node has meta entry "color" with type array

!has("color") // node doesn't have meta entry "color"
// similiarly there are && (and), || (or)

// node has meta entry "color" with type array and this array
// contains at least one element "red"
array_contains("color", "red")
```

available functionality
	- `has(entry)`
	- `has_<type>(entry)` [type from {string,array,int,date}]
	- `array_contains(entry, value)`
	- `equal(entry, value)` [required entry type deducted from value (?)]
	- `larger(entry, value)`
	- `smaller(entry, value)`
	- `string_matches(entry, regex-like)`
	- `array_contains_match(entry, regex-like)`

Wrap it up in some syntax:
All whitespace is ignored.
Startsymbol S, we use // as separating symbol since the syntax uses |.

```

// first rough try
S ::= A
A ::= !A // AB // (A) // E // P(<params>) // E=V // E:V // E<V // E>V
A2 ::= |A // ;A
E ::= <entryname>
V ::= <value>
T ::= <type>
P ::= <additional predicates>

// second try, should be LL(1), implement via recursive descent
S ::= AND
AND ::= OR(;OR)*
OR ::= NOT(|NOT)*
NOT ::= !ATOM // ATOM
ATOM ::= (AND) // E ATOMA
ATOMA ::= =U // :V
E ::= ID
U ::= 
V ::= VAL // <VAL> // V,V

ID ::= [identifier]
VAL ::= [value]

```

TODO: allow equal + match semantics for array?

Semantics:
	- precedence: '!' >> '|' >> ';' (or the other way around? confused)
	- A;B means A and B (use & instead?)
	- parantheses (as in (A)) can be used to overcome default precedence
	- E just means that entry E exists
	- P(<params>) means that the predicate is true for the given params
	  Used for additional (rarely used or custom) predicates.
	  [not sure if good idea, don't include it for now i guess]
	- E=V means that entry E exists (with same type as V) and equals V
	  V can be an array like this a,b,c
	  When entry is an array and V is just one value, will be true
	  if the entry has only that one entry.
	- E:V means that entry E matches value V
	  Always false if the entry is not of type string or array.
	  If entry is an array, will be true if one of its values
	  is exactly value. In this case V could also be a comma-separated
	  list, meaning that each value in the list must be contained
	  (exactly) in the array. To signal that a value should just be
	  matches, prefix it with an :

	  [NOPE, we should not do that for now]
	  This means "tags::no[dt]e,todo,:a[bB]c" means that tags
	  must contain:
	  	- value "todo"
		- a value that matches "nod[dt]e"
		- a value that matches "a[bB]c"

## Patterns

Many commands allow to filter nodes using a pattern.
A pattern basically allows to specify information about a node, like

 - its tags
 - name
 - content
 - id
 - various dates (created/modified/accessed)

For most information, various modes and combinations are supported
(regex/simple(full)/ simple(contain)/absent, etc).
Also supports basic logic combinations.

Example patterns:

```

"+t:a,b,c" => a nodes that has tags "a" and "b" and "c"
"+t:a,b,c;-t:d" => as above, but additionally must not have tag d
"+t:a|b" => must contain at least one of tags "a", "b"
"-t:a|b" => must contain at least one of tags "a", "b" not
"+t:<todo>" => must contain a tag that matches the "todo" regex
"-t:<todo>" => not tag must match "todo" regex
"t=a,b" => tags must be exactly "a" and "b"

"n=some name" => the name must be exactly some name
"+n:to*do" => name must match "to*do"
"-n:to*do" => name must not match "to*do"

"id:59..102" => id must be in range 59..102
"id:<343" => id must be <343
"id:>=343" => id must be >=343

```

Multiple of those patterns can be combined using a ';' like this:
"+n:todo;+t:todo" => name must contain todo and there must be "todo" tag.

### Commands [outdated, was a first concept]

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
