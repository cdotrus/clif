use std::env;

use cliproc::{cli, proc};
use cliproc::{Arg, Help};
use cliproc::{Cli, Command, ExitCode, Memory};

fn main() -> ExitCode {
    Cli::default().parse(env::args()).go::<Sum>()
}

type Digit = f32;

/// [Sum]` is a summation program that can add multiple unsigned 32-bit values together.
#[derive(PartialEq, Debug)]
struct Sum {
    nums: Vec<Digit>,
    verbose: bool,
}

impl Sum {
    /// Adds `lhs` and `rhs` together.
    fn run(&self) -> Digit {
        self.nums.iter().fold(Digit::default(), |acc, x| acc + x)
    }
}

// encoding, data, lang, symbols, tokens, tree, IR, repr
impl Command for Sum {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        // set short help text in case of an error
        cli.help(Help::default().text(HELP))?;
        Ok(Sum {
            verbose: cli.check(Arg::flag("verbose"))?,
            nums: cli.require_all(Arg::positional("num"))?,
        })
    }

    fn execute(self) -> proc::Result {
        let sum: Digit = self.run();
        if self.verbose == true {
            println!("{:?} = {}", self.nums, sum);
        } else {
            println!("{}", sum);
        }
        Ok(())
    }
}

const HELP: &str = "\
Computes the summation.

Usage:
    sum [options] <num>... 

Args:
    <num>       positive number

Options:
    --verbose   display computation work
";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn backend_logic() {
        let app = Sum {
            nums: vec![1.0, 2.0, 3.0],
            verbose: false,
        };

        assert_eq!(app.run(), 6.0);
    }
}
