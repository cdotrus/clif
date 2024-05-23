use crate::cli::Cli;
use crate::error::Error;

pub type CommandResult = Result<(), Box<dyn std::error::Error>>;
pub type CliResult<T> = Result<T, Error>;

pub trait Climb<T>: Sized {
    /// Collects tokens from the command-line interface to define a struct's fields.
    ///
    /// The recommended argument discovery order is
    /// 1. `flags`
    /// 2. `optionals`
    /// 3. `positionals`
    /// 4. `subcommands`
    ///
    /// It is considered a programmer's error if the arguments are not processed
    /// in the specified order by their type.
    fn from_cli(cli: &mut Cli) -> CliResult<Self>;

    /// Run the backend logic for this command.
    ///
    /// This function owns the self structure.
    fn execute(self, context: &T) -> CommandResult;
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

    impl Climb<()> for Add {
        fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, Error> {
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

        fn execute(self, _: &()) -> Result<(), Box<dyn std::error::Error>> {
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

    impl Climb<()> for Op {
        fn from_cli(cli: &mut Cli) -> Result<Self, Error> {
            let m = Ok(Op {
                force: cli.check_flag(Flag::new("force"))?,
                version: cli.check_flag(Flag::new("version"))?,
                command: cli.check_command(Positional::new("subcommand"))?,
            });
            cli.is_empty()?;
            m
        }

        fn execute(self, context: &()) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(command) = self.command {
                command.execute(context)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug, PartialEq)]
    enum OpSubcommand {
        Add(Add),
    }

    impl Climb<()> for OpSubcommand {
        fn from_cli(cli: &mut Cli) -> Result<Self, Error> {
            match cli.match_command(&["add", "mult", "sub"])?.as_ref() {
                "add" => Ok(OpSubcommand::Add(Add::from_cli(cli)?)),
                _ => panic!("an unimplemented command was passed through!"),
            }
        }

        fn execute(self, _: &()) -> CommandResult {
            match self {
                OpSubcommand::Add(c) => c.execute(&()),
            }
        }
    }

    #[test]
    fn make_add_command() {
        let mut cli = Cli::new().tokenize(args(vec!["add", "9", "10"]));
        let add = Add::from_cli(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 9,
                rhs: 10,
                force: false,
                verbose: false
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["add", "1", "4", "--verbose"]));
        let add = Add::from_cli(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 1,
                rhs: 4,
                force: false,
                verbose: true
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["add", "5", "--verbose", "2"]));
        let add = Add::from_cli(&mut cli).unwrap();
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
        let mut cli = Cli::new().tokenize(args(vec!["op", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli).unwrap();
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

        let mut cli = Cli::new().tokenize(args(vec!["op"]));
        let op = Op::from_cli(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: false,
                force: false,
                command: None
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["op", "--version", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli).unwrap();
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
        let mut cli = Cli::new().tokenize(args(vec!["op", "--verbose", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli);
        assert!(op.is_err());
    }

    #[test]
    #[should_panic]
    fn unimplemented_nested_command() {
        let mut cli = Cli::new().tokenize(args(vec!["op", "mult", "9", "10"]));
        let _ = Op::from_cli(&mut cli);
    }

    #[test]
    fn reuse_collected_arg() {
        let mut cli =
            Cli::new().tokenize(args(vec!["op", "--version", "add", "9", "10", "--force"]));
        let op = Op::from_cli(&mut cli).unwrap();
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
