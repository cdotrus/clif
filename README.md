# `cliproc`

[![Pipeline](https://github.com/cdotrus/cliproc/actions/workflows/pipeline.yml/badge.svg?branch=trunk)](https://github.com/cdotrus/cliproc/actions/workflows/pipeline.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/cliproc.svg)](https://crates.io/crates/cliproc)

This library provides support for fast, low-level, and configurable command-line processing.

``` toml
[dependencies]
cliproc = "1.0.0"
```

## Example

``` rust
use cliproc::{cli, proc};
use cliproc::{stage::Memory, Arg, Cli, Command, ExitCode, Help};
use std::env;

// 1. Define the struct and the data required to perform its task
struct Demo {
    name: String,
    count: Option<u8>,
}

// 2. Implement the `Command` trait to allow a struct to function as a command
impl Command for Demo {
    // 2a. Map the command-line data to the struct's data
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        cli.help(Help::with(HELP))?;
        Ok(Demo {
            name: cli.require(Arg::option("name").switch('n'))?,
            count: cli.get(Arg::option("count").switch('c'))?,
        })
    }

    // 2b. Process the struct's data to perform its task
    fn execute(self) -> proc::Result {
        for _ in 0..self.count.unwrap_or(1) {
            println!("Hello {}!", self.name);
        }
        Ok(())
    }
}

// 3. Build the command-line processor and run the command
fn main() -> ExitCode {
    Cli::default().parse(env::args()).go::<Demo>()
}

const HELP: &str = "\
A fast, low-level, and configurable command-line processor.

Usage:
    demo [options] --name <name>
    
Options:
    --name, -n <name>       Name of the person to greet              
    --count, -c <count>     Number of times to greet (default: 1)
    --help, -h              Print this help information and exit
";
```

See the [`examples/`](./examples/) folder for more demonstrations.

## Details

The command-line processor is divided into 3 stages: _build_, _ready_, and _memory_. It is implemented using the typestate pattern to enforce valid state operations and state transitions at compile-time.

1. _Build Stage_: The build stage provides methods to configure the command-line processor. This stage uses the builder pattern to set options.

2. _Ready Stage_: The ready stage provides methods to determine how to run the command-line processor.

3. _Memory Stage_: The memory stage provides methods to handle requests for data that is available from the command-line. This stage is the final stage of the command-line processor.

### Transitions

Parsing a set of arguments using `parse(...)` transitions the command-line processor from the build stage to the ready stage.

At the ready stage, the processor has two choices: _go_ or _save_:
- `go()` runs the processor to completion by transitioning to the memory stage and then handling calling the specified struct as a command with its implementation of the `Command` trait. This is the recommended choice for running the processor.

- `save()` puts off command interpretations and execution by only transitioning the processor to the memory stage. This allows the programmer to expliticly handle calling the specified struct as a command.

### Commands

The raw vector of strings received from the command-line is processed by translating the strings into tokens to be interpreted by a struct implementing the `Command` trait.

Any struct can be function as a command/subcommand as along as:
1. Each field's type in the struct implements the standard library's [`std::str::FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) trait.
2. The struct implements `cliproc`'s [`Command`](./src/proc.rs) (or [`Subcommand`](./src/proc.rs)) trait.

### Arguments

There are 4 supported types of arguments recognized by `cliproc`:
- _Flags_: boolean conditions (ex: `--verbose`)
- _Options_: arbitrary types for values specified with as a key/value pair (ex: `--output <file>`)
- _Positionals_: arbitrary types for values specified based upon position in the argument list (ex: `<name>`)
- _Subcommands_: arbitrary types for nesting commands that contain their own set of arguments (ex: `<command>`)

The command-line processor interprets arguments as they are provided. For this reason, there is a specific order in which a struct must handle its attributes according to which one of the argument types it requests data from.

Upon interpreting a command or subcommand, the _argument discovery order_ must follow:
1. Flags
2. Options
3. Positionals
4. Subcommands

Failure to specify the struct initialization in this order is a programmer's error and will result in a `panic!`.

## Features

The command-line processor has the ability to:  

- Accept long options for flags and options
    - `--verbose`, `--output a.out`

- Accept _switches_ (short options) for flags and options
    - `-v`, `-o a.out`

- Accept positional arguments
    - `main.rs`

- Nest commands within commands using subcommands
    - `calc add 10 20`, `calc mult 3 9`

- Accept attached value placement
    - `--output=a.out`, `-o=a.out`

- Aggregate switches
    - `-v -f`, `-vf`

- Capture variable instances of a flag with an optional maximum limit
    - `--verbose --verbose --verbose`

- Capture variable instances of an option (order-preserving) with an optional maximum limit
    - `--num 2 --num 17 --num 5`

- Capture variable positional calls for a single argument (order-preserving):
    - `10 20 30`

- Aggregate switches and assign a value to the final switch:
    - `-vfo=a.out`

- Prioritize flag for help information over any other parsing error

- Enable/disable a help flag with custom text entry WYSIWYG

- Retain parsing and argument handling knowledge to share arguments of a command with nested subcommands

- Detect misspelled words using dynamic programming approach for sequence alignment algorithm with configurable threshold for similarity comparison

- Verify there is no unused/unrecognized arguments before completing parsing

- Preserve unprocessed arguments that follow an empty flag `--`