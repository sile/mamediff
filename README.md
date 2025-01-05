mamediff
========

[![mamediff](https://img.shields.io/crates/v/mamediff.svg)](https://crates.io/crates/mamediff)
[![Documentation](https://docs.rs/mamediff/badge.svg)](https://docs.rs/mamediff)
[![Actions Status](https://github.com/sile/mamediff/workflows/CI/badge.svg)](https://github.com/sile/mamediff/actions)
![License](https://img.shields.io/crates/l/mamediff)

A TUI editor for unstaged and staged Git Diffs.
Inspired by [Magit], but designed to be significantly simpler and specialized for editing diffs.

**NOTE: This tool is still under development (version 0.1.0 is scheduled for release in January 2025).**

[Magit]: https://github.com/magit/magit

Installation
------------

```console
$ cargo install mamediff
```

Usage
-----

Just execute `mamediff` command within a Git directory. 
The available key bindings will be displayed in the top-right corner of the window.

```console
$ mamediff

```
