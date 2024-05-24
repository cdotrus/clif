use cliproc::{cli, proc};
use cliproc::{Cli, Command, ExitCode, Help, Optional};
use std::env;

// Fetch the command-line arguments to pass to the cli processor
fn main() -> ExitCode {
    Cli::default().tokenize(env::args()).go::<Demo>()
}

struct Demo {
    name: String,
    count: Option<u8>,
}

// Derive the `Command` trait to allow a struct to be built from the command-line
impl Command for Demo {
    // Translate the cli data into the backend data
    fn parse(cli: &mut Cli) -> cli::Result<Self> {
        cli.check_help(Help::default().text(HELP))?;
        Ok(Demo {
            name: cli.require_option(Optional::new("name").switch('n'))?,
            count: cli.check_option(Optional::new("count").switch('c'))?,
        })
    }

    // Run the backend process/logic
    fn execute(self) -> proc::Result {
        for _ in 0..self.count.unwrap_or(1) {
            println!("Hello {}!", self.name);
        }
        Ok(())
    }
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
