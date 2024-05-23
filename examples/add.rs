use std::env;
use std::error::Error;
use std::fmt::Display;

use cliproc::cli;
use cliproc::proc;
use cliproc::{Cli, Program};
use cliproc::{Flag, Help, Positional};

use std::process::ExitCode;

fn main() -> ExitCode {
    Cli::default().tokenize(env::args()).go::<(), Add>(())
}

#[derive(PartialEq, Debug)]
struct Add {
    left: u8,
    right: u8,
    verbose: bool,
}

impl Add {
    pub fn run(&self) -> u16 {
        self.left as u16 + self.right as u16
    }
}

#[derive(Debug, PartialEq)]
pub enum AddError {
    Overflow,
}

impl Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an overflow has occurred")
    }
}

impl Error for AddError {}

impl Program<()> for Add {
    fn parse(cli: &mut Cli) -> cli::Result<Self> {
        cli.check_help(Help::default().text(HELP))?;
        Ok(Add {
            verbose: cli.check_flag(Flag::new("verbose"))?,
            left: cli.require_positional(Positional::new("left"))?,
            right: cli.require_positional(Positional::new("right"))?,
        })
    }

    fn execute(self, _: &()) -> proc::Result {
        let sum = self.run();
        if sum > u8::MAX.into() {
            Err(AddError::Overflow)?
        } else {
            match self.verbose {
                true => println!("{} + {} = {}", self.left, self.right, sum),
                false => println!("{}", sum),
            }
            Ok(())
        }
    }
}

const HELP: &str = "\
Adds two numbers together.

Usage:
    add [options] <left> <right> 

Args:
    <left>       left-hand operand
    <right>       right-hand operand

Options:
    --verbose   display computation work
";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn backend_logic() {
        let prog = Add {
            left: 10,
            right: 9,
            verbose: false,
        };
        assert_eq!(prog.run(), 19);
    }
}
