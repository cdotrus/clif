use crate::cli;
use crate::cli::{Cli, Memory};

/// The return type for a [Command]'s execution process.
pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;

pub trait Command: Sized {
    /// Constructs the given struct by mapping the parsed representation
    /// of command-line inputs (tokens) into the appropriate data fields.
    ///
    /// The _argument discovery order_ must be preserved and upheld by the programmer:
    /// 1. `Flags`
    /// 2. `Optionals`
    /// 3. `Positionals`
    /// 4. `Subcommands`
    ///
    /// Failure to map the appropriate data fields in the correct order according to
    /// the method in how they recieve their data from the command-line is considered
    /// a programmer's error and will result in a panic!.
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self>;

    /// Processes the initialized struct and its defined data for an arbitrary
    /// task.
    ///
    /// A [Command] is considered a top-level process, and as such, cannot have
    /// a predefined context. For providing predefined contexts to commands, see
    /// the [Subcommand] trait.
    fn execute(self) -> Result;
}

pub trait Subcommand<T>: Sized {
    /// Constructs the given struct by mapping the parsed representation
    /// of command-line inputs (tokens) into the appropriate data fields.
    ///
    /// The _argument discovery order_ must be preserved and upheld by the programmer:
    /// 1. `Flags`
    /// 2. `Optionals`
    /// 3. `Positionals`
    /// 4. `Subcommands`
    ///
    /// Failure to map the appropriate data fields in the correct order according to
    /// the method in how they recieve their data from the command-line is considered
    /// a programmer's error and will result in a panic!.
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self>;

    /// Processes the initialized struct and its defined data for an arbitrary
    /// task.
    ///
    /// A [Subcommand] is considered a intermediate-level process, and as such,
    /// it requires a predefined context. For omitting a predefined context to
    /// a command, see the [Command] trait.
    fn execute(self, context: &T) -> Result;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{arg::*, help::Help};

    /// Helper test fn to write vec of &str as iterator for Cli parameter.
    fn args<'a>(args: Vec<&'a str>) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(args.into_iter().map(|f| f.to_string()).into_iter())
    }

    /// Example command to add two numbers together.
    #[derive(Debug, PartialEq)]
    struct Add {
        lhs: u32,
        rhs: u32,
        force: bool,
        verbose: bool,
    }

    impl Subcommand<()> for Add {
        fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
            cli.check_help(Help::new().text("Usage: add <lhs> <rhs> [--verbose]"))?;
            // the ability to "learn options" beforehand is possible, or can be skipped
            // "learn options" here (take in known args (as ref?))
            Ok(Add {
                force: cli.check_flag(Flag::new("force"))?,
                verbose: cli.check_flag(Flag::new("verbose"))?,
                lhs: cli.require_positional(Positional::new("lhs"))?,
                rhs: cli.require_positional(Positional::new("rhs"))?,
            })
        }

        fn execute(self, _: &()) -> Result {
            println!("{}", self.run());
            Ok(())
        }
    }

    impl Add {
        /// Simple fn to return an answer for the `Add` test command.
        fn run(&self) -> String {
            let sum = self.lhs + self.rhs;
            match self.verbose {
                true => format!("{} + {} = {}", self.lhs, self.rhs, sum),
                false => format!("{}", sum),
            }
        }
    }

    /// Tests a nested subcommand cli structure.
    #[derive(Debug, PartialEq)]
    struct Op {
        version: bool,
        force: bool,
        command: Option<OpSubcommand>,
    }

    impl Command for Op {
        fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
            let m = Ok(Op {
                force: cli.check_flag(Flag::new("force"))?,
                version: cli.check_flag(Flag::new("version"))?,
                command: cli.check_command(Positional::new("subcommand"))?,
            });
            cli.is_empty()?;
            m
        }

        fn execute(self) -> Result {
            if let Some(command) = self.command {
                command.execute(&())
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug, PartialEq)]
    enum OpSubcommand {
        Add(Add),
    }

    impl Subcommand<()> for OpSubcommand {
        fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
            match cli.match_command(&["add", "mult", "sub"])?.as_ref() {
                "add" => Ok(OpSubcommand::Add(Add::interpret(cli)?)),
                _ => panic!("an unimplemented command was passed through!"),
            }
        }

        fn execute(self, c: &()) -> Result {
            match self {
                OpSubcommand::Add(op) => op.execute(&c),
            }
        }
    }

    #[test]
    fn make_add_command() {
        let mut cli = Cli::new().parse(args(vec!["add", "9", "10"])).save();
        let add = Add::interpret(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 9,
                rhs: 10,
                force: false,
                verbose: false
            }
        );

        let mut cli = Cli::new()
            .parse(args(vec!["add", "1", "4", "--verbose"]))
            .save();
        let add = Add::interpret(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 1,
                rhs: 4,
                force: false,
                verbose: true
            }
        );

        let mut cli = Cli::new()
            .parse(args(vec!["add", "5", "--verbose", "2"]))
            .save();
        let add = Add::interpret(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 5,
                rhs: 2,
                force: false,
                verbose: true
            }
        );
    }

    #[test]
    fn nested_commands() {
        let mut cli = Cli::new().parse(args(vec!["op", "add", "9", "10"])).save();
        let op = Op::interpret(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                force: false,
                version: false,
                command: Some(OpSubcommand::Add(Add {
                    lhs: 9,
                    rhs: 10,
                    force: false,
                    verbose: false,
                }))
            }
        );

        let mut cli = Cli::new().parse(args(vec!["op"])).save();
        let op = Op::interpret(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: false,
                force: false,
                command: None
            }
        );

        let mut cli = Cli::new()
            .parse(args(vec!["op", "--version", "add", "9", "10"]))
            .save();
        let op = Op::interpret(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: true,
                force: false,
                command: Some(OpSubcommand::Add(Add {
                    lhs: 9,
                    rhs: 10,
                    force: false,
                    verbose: false,
                }))
            }
        );

        // out-of-context arg '--verbose' move it after 'add'
        let mut cli = Cli::new()
            .parse(args(vec!["op", "--verbose", "add", "9", "10"]))
            .save();
        let op = Op::interpret(&mut cli);
        assert!(op.is_err());
    }

    #[test]
    #[should_panic]
    fn unimplemented_nested_command() {
        let mut cli = Cli::new().parse(args(vec!["op", "mult", "9", "10"])).save();
        let _ = Op::interpret(&mut cli);
    }

    #[test]
    fn reuse_collected_arg() {
        let mut cli = Cli::new()
            .parse(args(vec!["op", "--version", "add", "9", "10", "--force"]))
            .save();
        let op = Op::interpret(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: true,
                // force is true here
                force: true,
                command: Some(OpSubcommand::Add(Add {
                    lhs: 9,
                    rhs: 10,
                    // force is true here as well
                    force: true,
                    verbose: false,
                }))
            }
        );
    }
}
