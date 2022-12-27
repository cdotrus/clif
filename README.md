# `clif`

### A lightweight and effective Command-Line Interface Framework.

## Features
- supports switches replacing flags/options
    - `--version`
    - `-v`
- supports combining multiple switches on single dash `-`
    - `-csi`
- supports options separating the flag and value by whitespace or equal sign `=`
    - `--output file.txt`
    - `--output=file.txt`
- supports nested subcommand architecture
    - `ops add --value 10 --value 20`
    - `ops mult --value 5 --value 4`
- supports positional arguments
    - `addrs 10 20`
- no macros: build your structs from scratch
- minimal dependencies (only directly uses the `colored` crate)
- uses dynamic programming sequence alignment algorithm to detect misspelled arguments (with configurable `threshold`)
- prioritizes help: if `-h` or `--help` is detected it will display quick help text regardless of other argument parsing errors (except errors on calling `--help` itself)
- complete control over the help text to display
- option to disable help flag (`--help`, `-h`) to be recognized as unknown flag


## Description

Arguments are learned as-they-go: the cli parser learns each argument upon checking it. For this reason, it is recommended to parse all flags and options before any positionals or subcommands.

For a type to be accepted by the command-line it must implement the `FromStr` trait.


## Examples

See the [examples/](./examples/) folder.