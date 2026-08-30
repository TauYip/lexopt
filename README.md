# Lexopt

Lexopt is an argument parser for Rust. It tries to have the simplest possible design that's still correct. It's so simple that it's a bit tedious to use.

Lexopt is:
- Small: one file, no dependencies, no macros. Easy to audit.
- Correct: standard conventions are supported and ambiguity is avoided.
- Imperative: options are returned as they are found, nothing is declared ahead of time.
- Minimalist: only basic functionality is provided.
- Unhelpful: there is no help generation and error messages often lack context.

## Examples
For basic usage, see [`examples/cargo.rs`](examples/cargo.rs).
For more details, see [`src/tests.rs`](src/tests.rs).

## Command line syntax
The following conventions are supported:
- Short options (`-q`)
- Long options (`--verbose`)
- `--` to mark the end of options
- `=` to separate options from values (`--option=value`, `-o=value`)
  - The nonstandard `-o=value` syntax can be disabled by `Parser::set_short_equals()`.
- Spaces to separate options from values (`--option value`, `-o value`)
- Unseparated short options (`-ovalue`)
- Combined short options (`-abc` to mean `-a -b -c`)
- Options with optional arguments (like GNU sed's `-i`, which can be used standalone or as `-iSUFFIX`) (`Parser::optional_value()`)

These are not supported out of the box:
- Single-dash long options (like find's `-name`)
- Abbreviated long options (GNU's getopt lets you write `--num` instead of `--number` if it can be expanded unambiguously)

## Why not?
This library may not be worth using if:
- You don't care about code size
- You do care about great error messages
- You hate boilerplate

## See also
- rustc's [`getopts`](https://docs.rs/getopts).
- [`clap`](https://docs.rs/clap).
