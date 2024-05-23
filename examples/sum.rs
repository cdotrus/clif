use std::env;

use cliproc::{cli, proc};
use cliproc::{Cli, ExitCode, Program};
use cliproc::{Flag, Help, Positional};

fn main() -> ExitCode {
    Cli::default().tokenize(env::args()).go::<(), Sum>(())
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

impl Program<()> for Sum {
    fn parse(cli: &mut Cli) -> cli::Result<Self> {
        // set short help text in case of an error
        cli.check_help(Help::default().text(HELP))?;
        Ok(Sum {
            verbose: cli.check_flag(Flag::new("verbose"))?,
            nums: cli.require_positional_all(Positional::new("num"))?,
        })
    }

    fn execute(self, _: &()) -> proc::Result {
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
