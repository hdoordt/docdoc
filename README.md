# DocDoc

Simple tool that stiches together a tree of text-based files. Currently only supports markdown.

## Installation
You'll need Rust and Cargo to build or install this tool.
You can find installation instructions for Rust and Cargo at <https://rustup.rs/>.

To install the latest release, just run

```
cargo install docdoc
```

## How it works

Create an entry file. Let's call it `entry.md`. It has the following content

```md
# My paper
A paper about DocDoc.

#[docdoc:path="./intro.md"]

#[docdoc:path="./conclusion.md"]
```

You'll notice the `#[docdoc:path="..."]` directives.
These directives tell DocDoc where to find the content to include.
DocDoc will replace the directives with the contents of the files in the paths, and resolve its includes recursively.

Let's add some other docs. First, `intro.md`:
```md
## Introduction

This is the introduction to my paper.
I like to keep things short.
```

And then `conclusion.md`:

```md
## Conclusion
So yeah, that was it. I had fun!
```

Now, lets have DocDoc stich this together:

```
docdoc -o output.md entry.md
```

And done! Open `output.md` to read the contents of the whole paper.
