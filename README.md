# Nodes

A simple note/thought/whatever keeping system.
Based upon a simple [specification](docs/spec.md) so that everyone
can easily develop other programs operating on the same set of data.

This repository contains a library for managing local nodes as well
as a cli. Currently in a very early alpha state, only the core functionality
is implemented.

## Show me something

[![asciicast](https://asciinema.org/a/pQBFdQlmw3my9eGasyQHf5Eit.png)](https://asciinema.org/a/pQBFdQlmw3my9eGasyQHf5Eit)

The example only shows very simple examples. You could now list nodes
for more complex patterns like `(tags:idea|tags:<[Tt]odo>);!color=red`
which means all nodes that have the tag "idea" or a tag that matches the
regex "[Tt]odo", that additionally don't have their color field set to
"red".

Note that all of these metadata types are not built-in for nodes, they will
only exist if you set/use them. You can generally associate whatever
metdata you want with nodes and then use it however you want.
