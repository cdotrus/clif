# `cliprs`

## A lightweight and flexible command-line argument parser written in Rust.


### Features
- no macros: build your structs from scratch
- minimal dependencies (only directly uses the `colored` crate)
- uses dynamic programming sequence alignment algorithm to detect misspelled arguments (with configurable `threshold`)
- prioritizes help: if `-h` or `--help` is detected it will display quick help text regardless of other argument parsing errors (except errors on calling `--help` itself)


### Description

Arguments are learned as-they-go: the cli parser learns each argument upon checking it. For this reason, it is recommended to parse all flags and options before any positionals or subcommands.

For a type to be accepted by the command-line it must implement the `FromStr` trait.


### Examples

See the [examples/](./examples/) folder.