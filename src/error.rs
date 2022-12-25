use crate::arg::Arg;
use crate::help::Help;
use colored::*;
use std::fmt::Display;

mod exit_code {
    pub const BAD: u8 = 101;
    pub const OKAY: u8 = 0;
}

type Value = String;
type Subcommand = String;
type Suggestion = String;
type PrevCommand = String;
type MaxCount = usize;
type CurCount = usize;
type Argument = String;
type Helper<'a> = Option<Help<'a>>;

#[derive(Debug)]
pub enum Error<'a> {
    /// argument, value, error message
    BadType(Arg<'a>, Value, Box<dyn std::error::Error>, Helper<'a>),
    /// argument, help
    MissingPositional(Arg<'a>, Helper<'a>),
    /// argument
    DuplicateOptions(Arg<'a>, Helper<'a>),
    /// argument
    ExpectingValue(Arg<'a>, Helper<'a>),
    /// argument, value
    UnexpectedValue(Arg<'a>, Value),
    /// argument, subcommand
    OutOfContextArgSuggest(Argument, Subcommand, Helper<'a>),
    /// argument
    UnexpectedArg(Argument, Helper<'a>),
    /// argument, suggestion
    SuggestArg(Argument, Suggestion),
    /// subcommand, suggestion
    SuggestSubcommand(Subcommand, Suggestion),
    /// argument, previous command
    UnknownSubcommand(Arg<'a>, PrevCommand, Helper<'a>),
    /// error message
    BrokenRule(Box<dyn std::error::Error>),
    /// quick help text
    Help(Helper<'a>),
    /// n, incorrect size, argument
    ExceedingMaxCount(MaxCount, CurCount, Arg<'a>),
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    BadType,
    MissingPositional,
    DuplicateOptions,
    ExpectingValue,
    UnexpectedValue,
    OutOfContextArgSuggest,
    UnexpectedArg,
    SuggestArg,
    SuggestSubcommand,
    UnknownSubcommand,
    BrokenRule,
    Help,
    ExceedingMaxCount
}

impl<'a> Error<'a> {
    /// Returns `OKAY_CODE` for help error and `BAD_CODE` otherwise.
    pub fn code(&self) -> u8 {
        match &self {
            Self::Help(_) => exit_code::OKAY,
            _ => exit_code::BAD,
        }
    }

    /// Casts the error to the quick help message if the error is for requesting help.
    pub fn as_quick_help(&self) -> Option<&str> {
        match &self {
            Self::Help(h) => Some(h.as_ref()?.get_quick_text()),
            _ => None,
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::ExceedingMaxCount(_, _, _) => ErrorKind::ExceedingMaxCount,
            Self::Help(_) => ErrorKind::Help,
            Self::SuggestArg(_, _) =>ErrorKind::SuggestArg,
            Self::SuggestSubcommand(_, _) =>ErrorKind::SuggestSubcommand,
            Self::OutOfContextArgSuggest(_, _, _) => ErrorKind::OutOfContextArgSuggest,
            Self::BadType(_, _, _, _) => ErrorKind::BadType,
            Self::MissingPositional(_, _) => ErrorKind::MissingPositional,
            Self::DuplicateOptions(_, _) => ErrorKind::DuplicateOptions,
            Self::ExpectingValue(_, _) => ErrorKind::ExpectingValue,
            Self::UnexpectedValue(_, _) => ErrorKind::UnexpectedValue,
            Self::UnexpectedArg(_, _) =>ErrorKind::UnexpectedArg,
            Self::UnknownSubcommand(_, _, _) => ErrorKind::UnknownSubcommand,
            Self::BrokenRule(_) => ErrorKind::BrokenRule,
        }
    }
}

impl<'a> std::error::Error for Error<'a> {}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // body
        match self {
            Self::ExceedingMaxCount(n, s, o) => write!(f, "option '{}' was requested {} times, but cannot exceed {}", o, s, n),
            Self::Help(h) => write!(f, "{}", h.as_ref().unwrap_or(&Help::new()).get_quick_text()),
            Self::SuggestArg(a, sug) => write!(f, "unknown argument '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::SuggestSubcommand(a, sug) => write!(f, "unknown subcommand '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::OutOfContextArgSuggest(o, cmd, _) => write!(f, "argument '{}' is unknown or invalid in the current context\n\nMaybe move it after '{}'?", o.yellow(), cmd.green()),
            Self::BadType(a, v, e, _) => write!(f, "argument '{}' did not process '{}' due to {}", a.to_string().yellow(), v.blue(), e),
            Self::MissingPositional(p, help) => write!(f, "missing required argument '{}'{}", p, match help.as_ref().unwrap_or(&Help::new()).get_usage() { Some(m) => { "\n\n".to_owned() + m } None => { "".to_owned() } }),
            Self::DuplicateOptions(o, _) => write!(f, "option '{}' was requested more than once, but can only be supplied once", o.to_string().yellow()),
            Self::ExpectingValue(x, _) => write!(f, "option '{}' expects a value but none was supplied", x.to_string().yellow()),
            Self::UnexpectedValue(x, s) => write!(f, "flag '{}' cannot accept values but one was supplied \"{}\"", x.to_string().yellow(), s),
            Self::UnexpectedArg(s, _) => write!(f, "unknown argument '{}'", s.yellow()),
            Self::UnknownSubcommand(c, a, _) => write!(f, "'{}' is not a valid subcommand for {}", a, c.to_string().yellow()),
            Self::BrokenRule(r) => write!(f, "a rule conflict occurred from {}", r),
        }?;
        // footer
        match self {
            Self::OutOfContextArgSuggest(_, _, he)
            | Self::BadType(_, _, _, he)
            | Self::MissingPositional(_, he)
            | Self::DuplicateOptions(_, he)
            | Self::ExpectingValue(_, he)
            | Self::UnexpectedArg(_, he)
            | Self::UnknownSubcommand(_, _, he) => {
                if let Some(help) = he {
                    write!(f, "\n\nFor more information try {}", help.get_flag())?
                }
            }
            _ => (),
        }
        Ok(())
    }
}
