mamediff
========

[![mamediff](https://img.shields.io/crates/v/mamediff.svg)](https://crates.io/crates/mamediff)
[![Documentation](https://docs.rs/mamediff/badge.svg)](https://docs.rs/mamediff)
[![Actions Status](https://github.com/sile/mamediff/workflows/CI/badge.svg)](https://github.com/sile/mamediff/actions)
![License](https://img.shields.io/crates/l/mamediff)

A TUI editor for managing unstaged and staged Git diffs.
Inspired by [Magit], this tool focuses on providing a simpler, specialized interface for staging, unstaging, and discarding diffs.

[Magit]: https://github.com/magit/magit

![mamediff](mamediff.gif)

Installation
------------

### Pre-built binaries

Pre-built binaries for Linux and MacOS are available in [the releases page](https://github.com/sile/mamediff/releases).

```console
// An example to download the binary for Linux.
$ VERSION=0.1.3
$ curl -L https://github.com/sile/mamediff/releases/download/v${VERSION}/mamediff-${VERSION}.x86_64-unknown-linux-musl -o mamediff
$ chmod +x mamediff
$ ./mamediff -h
```

### With [Cargo](https://doc.rust-lang.org/cargo/)

If you have installed `cargo` (the package manager for Rust), you can install `mamediff` with the following command:

```console
$ cargo install mamediff
$ mamediff -h
```

Usage
-----

Just execute `mamediff` command within a Git directory.
The available key bindings will be displayed in the top-right corner of the window.

```console
$ mamediff
->| Unstaged changes (1 files)                     | (q)uit [ESC,C-c]
  :   modified src/main.rs (1 chunks, -1 +1 lines) | (↓)        [C-n]
  :     @@ -1,3 +1,3 @@                            | (→)        [C-f]
  :        fn main() {                             | (t)oggle   [TAB]
  :       -    println!("Hello, World!");          | (s)tage
  :       +    println!("Hello, mamediff!");       | (D)iscard
  :        }                                       +---- (h)ide -----
  | Staged changes (0 files)
```

You Might Also Be Interested In
-------------------------------

- [mamegrep](https://github.com/sile/mamegrep): A TUI tool for `$ git grep`
