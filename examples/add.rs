use yacli::command::{FromCli, Command, Runner};
use yacli::cli::Cli;
use yacli::errors::CliError;
use yacli::arg::*;
use std::env::args;

fn main() {
    std::process::exit(go() as i32)
}

/// Glues the interface layer and backend logic for a smooth hand-off of data.
fn go() -> u8 {
    // parse the command-line arguments
    let mut cli = Cli::new()
        .threshold(4)
        .tokenize(args());

    match Radd::from_cli(&mut cli) {
        // construct the application
        Ok(app) => { 
            std::mem::drop(cli); 
            app.exec(&()) 
        },
        // report cli error
        Err(err) => { 
            match err.code() { 
                // handle help information returning a zero exit code
                0 => println!("{}", err.to_string()),
                // display the cli parsing error message
                _ => eprintln!("{}", err.to_string())
            }
            err.code()
        }
    }
}

/// `Radd` is 'rust-add' that can add two unsigned 8-bit values together.
#[derive(PartialEq, Debug)]
struct Radd {
    lhs: u8,
    rhs: u8, 
    verbose: bool,
}

impl Radd {
    /// Adds `lhs` and `rhs` together.
    fn run(&self) -> u16 {
        self.lhs as u16 + self.rhs as u16
    }
}

// enforce the `Radd` struct to implement `FromCli` and `Command<T>` traits
impl Runner<()> for Radd {}

impl FromCli for Radd {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, CliError<'c>> where Self: Sized {
        // set short help text in case of an error
        cli.help(HELP, Some(USAGE_LINE))?;
        let radd = Radd {
            verbose: cli.check_flag(Flag::new("verbose"))?,
            lhs: cli.require_positional(Positional::new("lhs"))?,
            rhs: cli.require_positional(Positional::new("rhs"))?,
        };
        // verify the cli has no additional arguments if this is the top-level command being parsed
        cli.is_empty()?;
        Ok(radd)
    }
}

impl Command<()> for Radd {
    type Status = u8;

    fn exec(&self, _: &()) -> Self::Status {
        let sum: u16 = self.run();
        if self.verbose == true {
            println!("{} + {} = {}", self.lhs, self.rhs, sum);
        } else {
            println!("{}", sum);
        }
        0
    }
}

const USAGE_LINE: usize = 3;

const HELP: &str = "\
Adds two numbers together.

Usage:
    radd [options] <lhs> <rhs> 

Args:
    <lhs>       left-hand operand
    <rhs>       right-hand operand

Options:
    --verbose   display computation work
";

#[cfg(test)]
mod test {
    use super::*; 

    #[test]
    fn backend_logic() {
        let app = Radd {
            lhs: 10,
            rhs: 9,
            verbose: false,
        };

        assert_eq!(app.run(), 19);
    }
}