use cliproc::{cli, proc};
use cliproc::{Arg, Cli, Command, ExitCode, Help, Memory, Subcommand};
use std::env;

#[derive(Debug, PartialEq)]
struct Calc {
    version: bool,
    force: bool,
    op: Option<Operation>,
}

impl Command for Calc {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        Ok(Calc {
            force: cli.check(Arg::flag("force"))?,
            version: cli.check(Arg::flag("version"))?,
            op: cli.nest(Arg::subcommand("operation"))?,
        })
    }

    fn execute(self) -> proc::Result {
        if self.version == true {
            println!("1.0.0");
            Ok(())
        } else {
            if self.force == true {
                println!("Force enabled in base command!");
            }
            if let Some(op) = self.op {
                op.execute(&())
            } else {
                Ok(())
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum Operation {
    Add(Add),
    Mult(Mult),
}

impl Subcommand<()> for Operation {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        match cli.select(&["add", "mult"])?.as_ref() {
            "add" => Ok(Operation::Add(Add::interpret(cli)?)),
            "mult" => Ok(Operation::Mult(Mult::interpret(cli)?)),
            _ => panic!("an unimplemented command was passed through!"),
        }
    }

    fn execute(self, c: &()) -> proc::Result {
        match self {
            Operation::Add(op) => op.execute(&c),
            Operation::Mult(op) => op.execute(&c),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Add {
    lhs: u32,
    rhs: u32,
    force: bool,
    verbose: bool,
}

impl Subcommand<()> for Add {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        cli.help(Help::new().text("Addition"))?;
        Ok(Add {
            force: cli.check(Arg::flag("force"))?,
            verbose: cli.check(Arg::flag("verbose"))?,
            lhs: cli.require(Arg::positional("lhs"))?,
            rhs: cli.require(Arg::positional("rhs"))?,
        })
    }

    fn execute(self, _: &()) -> proc::Result {
        let sum = self.lhs + self.rhs;
        if self.force == true {
            println!("Force enabled in subcommand!");
        }
        match self.verbose {
            true => println!("{} + {} = {}", self.lhs, self.rhs, sum),
            false => println!("{}", sum),
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct Mult {
    lhs: f32,
    rhs: f32,
}

impl Subcommand<()> for Mult {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        cli.help(Help::new().text("Multiplication"))?;
        Ok(Mult {
            lhs: cli.require(Arg::positional("lhs"))?,
            rhs: cli.require(Arg::positional("rhs"))?,
        })
    }

    fn execute(self, _: &()) -> proc::Result {
        let prod = self.lhs * self.rhs;
        println!("{} x {} = {}", self.lhs, self.rhs, prod);
        Ok(())
    }
}

fn main() -> ExitCode {
    Cli::default().parse(env::args()).go::<Calc>()
}
