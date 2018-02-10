# TODO list for nodes:

## Stuff until v0.1:

- [x] cleanup implementation (error handling, no hacks, modular!)
- [x] write a matching first specification
- [ ] fix 'nodes ref' (especially error handling)

## Specific todos:

- [x] config: ls default count
- [ ] config (and ls command): which data to output/summary (+format)?

## Later, def. after v0.1:

- [ ] first try of file type parsing
- [ ] command to modify meta data (without using 'edit --meta')
- [ ] shortcuts for meta fields (like n for name or t for tags. c for content?)

## More idea like:

- [ ] add more config options
  - [ ] custom tracking of accessed/modified nodes
  - [ ] own, internal (per-storage?) node-list
    - [ ] would allow to access them e.g. from javascript
- [ ] Allow @path qualifier in programs args, make it work for config/meta
- [ ] toy around with additional node metadata
  - [ ] color
  - [ ] custom metadata
- [ ] node links
  - [ ] in which way to retrieve/set them?
- [ ] general extension framwork (specification)
  - [ ] anything to specify at all?
- [ ] mutliple file types
  - [x] way to define editors/viewers/previews in config
  	 - [ ] also custom editor type? like 'nodes edit --category mycat 42'
	       that will use the programs specified in mycat?
  - [ ] mime types?, we could use libmagic
  - [ ] node collections (ordered? lookup?) (how of use?)
  - [ ] node template types?
  - [ ] how can extensions use/define own types?
- [x] clean up implementation (i.e. make real library)
  - [ ] figure out what to move to library. General util functions like
        read_summary, short_string or list_node are useful in the library
- [x] multiple storages (as a specification)
- [ ] advanced find/search patterns
  - [x] multiple tags (absence and presence)
  - [ ] content in nodes (absence and presence)
  - [ ] in/outside a given date range
  - [x] content in name (absense and presence)
  - [x] support regex
- [x] more general metadata way (allow user to set metadata -> see below)
- [ ] manging utility
  - [x] easier multi-delete
  - [ ] multi edit/show?
  - [ ] node to filesystem file (reverse of add)
  - [x] get node path
  - [ ] better way to add/modify meta data
    - [ ] add tags
	- [ ] change name
	- [ ] add custom metadata entries
- [x] allow to specify meta information (for text nodes) on creation time
  - [x] something like "-- nodes-tags: tag1 tag2 tag3" at the end of the file
- [x] allow fast, inline node creation (like "nodes o -m 'some text'")
- [ ] automatically parse node type (image? url?)
- [ ] make things modular by simply piping multiple node commands into each
      other?
  - [ ] like `nodes find tags:remove | nodes rm`

Just a dummy node, the real todo list resides in the node repo for now.
With multiple storage spaces, this could change (and with collections/links
this could be made into a real list consisting of multiple items).
