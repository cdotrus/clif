# Changelog

## 2.0.0 - unreleased

### Features
- Enforces 3-stage processor design (Build, Ready, Memory) using typestate pattern
- Enforces builder pattern for the Build stage in configuring the processor
- Refactors `Cli` API into much cleaner and more intuitive names
- Refactors which modules and structs to expose to programmer
- Refactors `Help` struct with cleaner API

## 1.0.0

### Features 
- Long options (flags/options): `--verbose`
- Short options (switches): `-v`
- Positional value placement: `--output a.out`, `-o a.out`
- Attached value placement: `--output=a.out`, `-o=a.out`
- Combine switches: `-rf`
- Capture variable flag calls: `--inc --inc --inc...`
- Capture variable option calls: `--op 10 --op 20 --op 30`
- Capture variable positional calls: `10 20 30`
- Capture variable limited user-defined caps for flag and option calls: `--verbose --verbose --verbose` (ex: up to 3)
- Combine switches and give value to final switch: `-rfo=a.out`
- Ability to prioritize help over any other error
- Ability to enable/disable a help flag with custom long/short flag names
- Subcommand support: `calc add 10 20`, `calc mult 10 20`
- Arguments can be reused in both structs for overall command and nested subcommand
- Uses dynamic programming sequence alignment algorithm to detect misspelled arguments (with configurable `threshold` for string comparison)
- Ability to set user-defined help text before parsing
- Ability to verify there are no unused arguments passed in the cli
- Verifies all argument type conversions are accepted during parsing
- Preserves unprocessed arguments found after an empty flag `--` symbol