# Nodes specification v0.1

This is the first alpha specification of the nodes system.
It has to be seen as highly unstable as many aspects of the specification
are likely to change in the next versions.

## Overview and motivation

The goal is a simple yet extensible system for personal notes, thoughts,
snippets, collections and general files -- the idea of a node.
Instead of just writing a tool, the system is described by a small
spefication since one of the goals of nodes is to make the nodes
accessible in many ways, on many platforms and in which way ever one likes
to do.

## Central configuration

The central nodes config file is placed at $HOME/.config/nodes/config,
where $HOME is the users home path.
It has the toml file format and per standard the following fields:

- "storage.default": Name of the default storage (string)
- "storage.storages": Array of tables that describe the availble storages
  - ".name": The name of a storage (string)
  - ".path": The file path of the storage (string)

Extensions/tools can add/load additional config values to/from this file.
By default (e.g. when the config file does not exist), the initial
default node storage is used (also set as default storage), located
at $HOME/.local/share/nodes.
See the storages section for more information about storages.

## Storages

A node storage is an abstract location that contains nodes.
On a system, every storage has a unique name.
Usually these storages are filepaths but extensions/tools may
provide custom storage types (like e.g. cloud-based storages).

A storage always has the following layout:

. The storage root folder
|
|-- storage
|-- nodes/
|-- meta/

The storage file contains information about the storage.
It has the toml file format and the key "last_id" is always
set to the last unique id node used for a node (type integer).
When a new node is created, the value must be increased.

The nodes/ folder contains the node files. Every file has just
the name of the nodes' id.

The meta/ folder contains the metadat files associated with the nodes. 
Every file has just the name of the nodes' id and the toml file format.
Programs/extensions/users are free to add any values to these files.

## Node

A node is a piece of information.
Nothing else of its shape is specified, its definition and usage
is mainly up to the user.
Per specification there are only two things every node has: 

- an unique id (unique per storage)
- a set of associated metadata

The unique id is used as reference to the node while the metadata
can be used freely by the user.
Depending on the tool and platform, the id might or might not be exposed
to the end user.
