# `yacli`

## A lightweight command-line argument parser.


### Features
- no macros: build your structs from scratch
- minimal dependencies (only directly uses the `colored` crate)
- uses dynamic programming sequence alignment algorithm to detect misspelled arguments (with configurable `threshold`)
- prioritizes help: if `-h` or `--help` is detected it will display quick help text regardless of other argument parsing errors