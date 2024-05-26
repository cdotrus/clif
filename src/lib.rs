mod arg;
mod error;
mod help;
mod seqalin;

pub mod cli;
pub mod proc;

pub use arg::{Arg, Flag, Optional, Positional};
pub use cli::states::{Build, Memory, Ready};
pub use cli::Cli;
pub use help::Help;
pub use proc::{Command, Subcommand};
pub use std::process::ExitCode;

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper test `fn` to write vec of &str as iterator for Cli parameter.
    fn args<'a>(args: Vec<&'a str>) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(args.into_iter().map(|f| f.to_string()).into_iter())
    }

    mod add {
        use super::*;

        mod ok {
            use super::*;

            #[derive(PartialEq, Debug)]
            struct Add {
                lhs: u8,
                rhs: u8,
                verbose: bool,
            }

            impl Add {
                fn run(&self) -> u16 {
                    self.lhs as u16 + self.rhs as u16
                }
            }

            impl Command for Add {
                fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
                    // set help text in case of an error
                    cli.help(Help::default().text(String::new()))?;
                    let radd = Add {
                        verbose: cli.check(Arg::flag("verbose"))?,
                        lhs: cli.require(Arg::positional("lhs"))?,
                        rhs: cli.require(Arg::positional("rhs"))?,
                    };
                    // optional: verify the cli has no additional arguments if this is the top-level command being parsed
                    cli.is_empty()?;
                    Ok(radd)
                }

                fn execute(self) -> proc::Result {
                    let sum: u16 = self.run();
                    if self.verbose == true {
                        println!("{} + {} = {}", self.lhs, self.rhs, sum);
                    } else {
                        println!("{}", sum);
                    }
                    Ok(())
                }
            }

            #[test]
            fn it_add_program() {
                let mut cli = Cli::new()
                    .threshold(4)
                    .parse(args(vec!["add", "45", "17"]))
                    .save();
                let program = Add::interpret(&mut cli).unwrap();
                std::mem::drop(cli);
                assert_eq!(program.run(), 62);
            }
        }

        mod bad {
            use super::*;

            #[derive(PartialEq, Debug)]
            struct Add {
                lhs: u8,
                rhs: u8,
                verbose: bool,
            }

            impl Add {
                fn run(&self) -> u16 {
                    self.lhs as u16 + self.rhs as u16
                }
            }

            impl Command for Add {
                fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
                    // set help text in case of an error
                    cli.help(Help::default().text(String::new()))?;
                    let radd = Add {
                        lhs: cli.require(Arg::positional("lhs"))?,
                        verbose: cli.check(Arg::flag("verbose"))?,
                        rhs: cli.require(Arg::positional("rhs"))?,
                    };
                    // optional: verify the cli has no additional arguments if this is the top-level command being parsed
                    cli.is_empty()?;
                    Ok(radd)
                }

                fn execute(self) -> proc::Result {
                    let sum: u16 = self.run();
                    if self.verbose == true {
                        println!("{} + {} = {}", self.lhs, self.rhs, sum);
                    } else {
                        println!("{}", sum);
                    }
                    Ok(())
                }
            }

            #[test]
            #[should_panic]
            fn it_add_program() {
                let mut cli = Cli::new()
                    .threshold(4)
                    .parse(args(vec!["add", "45", "17"]))
                    .save();
                let _ = Add::interpret(&mut cli);
            }
        }
    }
}
