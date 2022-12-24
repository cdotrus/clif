use crate::arg::Arg;
use std::fmt::Display;
use std::error::Error;
use colored::*;
use crate::help::Help;

mod exit_code {
    pub const BAD: u8 = 101;
    pub const OKAY: u8 = 0;
}

type Value = String;
type ErrMessage = String;
type Subcommand = String;
type Suggestion = String;
type PrevCommand = String;
type MaxCount = usize;
type CurCount = usize;
type Argument = String;

#[derive(Debug, PartialEq)]
pub enum CliError<'a> {
    /// argument, value, error message 
    BadType(Arg<'a>, Value, ErrMessage),
    /// argument, help
    MissingPositional(Arg<'a>, Help<'a>),
    /// argument
    DuplicateOptions(Arg<'a>),
    /// argument
    ExpectingValue(Arg<'a>),
    /// argument, value
    UnexpectedValue(Arg<'a>, Value),
    /// argument, subcommand
    OutOfContextArgSuggest(Argument, Subcommand),
    /// argument
    UnexpectedArg(Argument),
    /// argument, suggestion
    SuggestArg(Argument, Suggestion),
    /// subcommand, suggestion
    SuggestSubcommand(Subcommand, Suggestion),
    /// argument, previous command
    UnknownSubcommand(Arg<'a>, PrevCommand),
    /// error message
    BrokenRule(ErrMessage),
    /// quick help text
    Help(Help<'a>),
    /// n, incorrect size, argument
    ExceedingMaxCount(MaxCount, CurCount, Arg<'a>),
}

impl<'a> CliError<'a> {
    /// Returns `OKAY_CODE` for help error and `BAD_CODE` otherwise.
    pub fn code(&self) -> u8 {
        match &self {
            Self::Help(_) => exit_code::OKAY,
            _ => exit_code::BAD
        }
    }

    /// Displays the cli error to the console and returns its respective error
    /// code.
    pub fn explain(&self) -> u8 {
        match &self {
            // handle help information returning a zero exit code
            Self::Help(_) => println!("{}", self.to_string()),
            // display the cli parsing error message
            _ => eprintln!("{}", self.to_string()),
        }
        self.code()
    }
}

impl<'a> Error for CliError<'a> {}

impl<'a> Display for CliError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> { 
        // header
        match self {
            Self::Help(_) => (),
            _ => write!(f, "{}", "error: ".red().bold())?
        };
        // body
        match self {
            Self::ExceedingMaxCount(n, s, o) => write!(f, "option '{}' was requested {} times, but cannot exceed {}", o, s, n),
            Self::Help(h) => write!(f, "{}", h.get_quick_text()),
            Self::SuggestArg(a, sug) => write!(f, "unknown argument '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::SuggestSubcommand(a, sug) => write!(f, "unknown subcommand '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::OutOfContextArgSuggest(o, cmd) => write!(f, "argument '{}' is unknown, or invalid in the current context\n\nMaybe move it after '{}'?", o.yellow(), cmd.green()),
            Self::BadType(a, v, e) => write!(f, "argument '{}' did not process '{}' due to {}", a.to_string().yellow(), v.blue(), e),
            Self::MissingPositional(p, help) => write!(f, "missing required argument '{}'\n\n{}", p, help.get_usage().unwrap_or("")),
            Self::DuplicateOptions(o) => write!(f, "option '{}' was requested more than once, but can only be supplied once", o.to_string().yellow()),
            Self::ExpectingValue(x) => write!(f, "option '{}' expects a value but none was supplied", x.to_string().yellow()),
            Self::UnexpectedValue(x, s) => write!(f, "flag '{}' cannot accept values but one was supplied \"{}\"", x.to_string().yellow(), s),
            Self::UnexpectedArg(s) => write!(f, "unknown argument '{}'", s.yellow()),
            Self::UnknownSubcommand(c, a) => write!(f, "'{}' is not a valid subcommand for {}", a, c.to_string().yellow()),
            Self::BrokenRule(r) => write!(f, "a rule conflict occurred from {}", r),
        }?;
        // footer
        match self {
            Self::OutOfContextArgSuggest(_, _) | 
            Self::BadType(_, _, _) |
            Self::MissingPositional(_, _) |
            Self::DuplicateOptions(_) |
            Self::ExpectingValue(_) |
            Self::UnexpectedArg(_) |
            Self::UnknownSubcommand(_, _) => write!(f, "\n\nFor more information try {}", "--help".green())?,
            _ => ()
        }
        Ok(())
    }
}