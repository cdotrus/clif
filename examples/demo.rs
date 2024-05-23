use cliproc::*;
use std::env;

fn main() -> ExitCode {
    Cli::default().tokenize(env::args()).go::<Demo>()
}

struct Demo {
    name: String,
    count: u8,
}

impl Program for Demo {
    fn parse(cli: &mut Cli) -> cli::Result<Self> {
        cli.check_help(Help::default().text(HELP))?;
        Ok(Demo {
            name: cli.require_positional(Positional::new("name"))?,
            count: cli
                .check_option(Optional::new("count").switch('c'))?
                .unwrap_or(1),
        })
    }

    fn execute(self) -> proc::Result {
        for _ in 0..self.count {
            println!("Hello {}!", self.name);
        }
        Ok(())
    }
}

const HELP: &str = "\
A fast, low-level, and configurable command-line processor.

Usage:
    demo [options] <name>

Arguments:
    <name>                  Name of the person to greet
    
Options:
    <name>                  
    --count, -c <count>     Number of times to greet (default: 1)
    --help, -h              Print this help information and exit
";
