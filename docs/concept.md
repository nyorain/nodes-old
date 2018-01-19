- Base: file format specification
	- What is a "node"
	- extensible
	- does not specify its contents, rather a header or some common ground
- Can be extended with plugins and stuff
	- everything is programmable, automatable
- the result/goal:
	- data organization in a way that is easy to process/create for humans
	  as well as for computers. The default filesystem mechanism is neither,
	  it's only a hirachy.
	- other explanation approach: Extending (replacing) the folder-like
	  filesystem mechanism for OUR data.
		- connecting data, files
		- allowing to add file metadata
		- organizing files in a different way than in folders
		- don't think of files, think of nodes
			- like fragments of a file
			- can be easily moved around, used wherever needed
		- not really replacing in general, only for this specific
		  (personal data, concepts, ideas, notes etc) usage
- Does not assume anything. Might be used as a cloud/web service or only locally,
  implemented however someone wants to implement it.
  It's only a specification. Not trying to reinvent the wheel, but do
  improve it.

Example: linking between nodes

- Different ways to address a node
	- "name" -> scope-dependent name lookup
	- "/sub/name" -> unique-id like a filepath
	- "service:name" -> fetching the node from another service
		- could be used to e.g. get nodes directly from the internet
- Of course also links to other stuff than nodes are allowed
	- files, urls etc
	- also possible to link actions/commands of scripts/extensions?

```node
This is a small node.
It could e.g. be a list of some stuff i made in the last month [1].
	- A picture [2]
	- A story [3]
	- Maybe a file link to this document? [4]
		- yeah... it's moved by now so not sure such links are probably
		  not a good idea?

[1](october)
[2](mypic)
[3](mystory)
[4](file://~/programming/nodes/concept.md)
```

- goal: integrate well with text-editors

So: what is a node?
-------------------

- a piece of information
	- can link to other nodes
	- can consist of other nodes, like an album or a todo list
	- can be an atomic piece of information like a picture, text, song or url
	- or a code snippet
	- or some custom type (like a conversation, a memory, a book chapter,
	  a drawing, a film or video, a protocol, whatever basically)

basic operations:

- you can create a new node in different ways
	- create node and open it in text editor
	- copy a file from the internet or the filesystem
	- link to a file or a website (hard/softlink, url or sth)
	- create a node with the given text content
	- create a custom node type (in editor?)
- you can delete a node
	- [if desired, configurable] will be moved to trash first
- you can shows all nodes in various ways/with various filters
	- like in a hierachy, list or graph
	- filter: date/name/tag/other methods like color or tone
- track history
	- e.g. easily refer to last created/edited nodes
- tag nodes, associate metadata
- somehow operate on/between nodes
	- show outgoing links of node
	- show ingoing link of a node
	- add abstract links

advanced (later on, maybe, mainly just ideas):

- you can install plugins & extensions, register handlers basically?
	- probably not do anything like it, just use different programs then
	- no specification for plugin or extensions
- you can import a set of nodes
- you can export all nodes in various ways
- you can connect to an internet instance to synchronize
- you can integrate your nodes with git (or other history, sync system)

## Note type ideas

- playlist. Can be exported to a real playlist (with various settings)
- photo album
- ideas/concepts etc, general short thoughts
- todo list
- project, as in programming project
