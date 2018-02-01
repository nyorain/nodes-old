# Default implementation [ideas]

The default implementation allows to deal with nodes from the command line.
It has no graphical user interface but can be connected to external programs
like an editor, browser or image/video/audio programs.

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
