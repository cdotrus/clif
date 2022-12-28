use crate::arg::Arg;
use crate::help::Help;
use std::fmt::Display;

mod exit_code {
    pub const BAD: u8 = 101;
    pub const OKAY: u8 = 0;
}

type Value = String;
type Subcommand = String;
type Suggestion = String;
type MaxCount = usize;
type CurCount = usize;
type Argument = String;

#[derive(Debug)]
pub struct Error<'a> {
    context: ErrorContext<'a>,
    help: Option<Help<'a>>,
    kind: ErrorKind,
}

impl<'a> Error<'a> {
    /// Creates a new error.
    pub fn new(help: Option<Help<'a>>, kind: ErrorKind, context: ErrorContext<'a>) -> Self {
        Self {
            help: help,
            kind: kind,
            context: context,
        }
    }

    /// Casts the error to the quick help message if the error is for requesting help.
    pub fn as_quick_help(&self) -> Option<&str> {
        match &self.kind {
            ErrorKind::Help => Some(self.help.as_ref()?.get_quick_text()),
            _ => None,
        }
    }

    // Returns the kind of error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns `OKAY_CODE` for help error and `BAD_CODE` otherwise.
    pub fn code(&self) -> u8 {
        match &self.kind {
            ErrorKind::Help => exit_code::OKAY,
            _ => exit_code::BAD,
        }
    }

    /// References the surrounding structs for the given error.
    pub fn context(&self) -> &ErrorContext {
        &self.context
    }

    /// Constructs a simple help tip to insert into an error message if help exists.
    pub fn help_tip(&self) -> Option<String> {
        Some(format!("\n\nFor more information try {}", self.help.as_ref()?.get_flag()))
    }
}

type ParseError = Box<dyn std::error::Error>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorContext<'a> {
    ExceededThreshold(Arg<'a>, CurCount, MaxCount),
    FailedArg(Arg<'a>),
    UnexpectedValue(Arg<'a>, Value),
    FailedCast(Arg<'a>, Value, ParseError),
    OutofContextArgSuggest(Argument, Subcommand),
    UnexpectedArg(Argument),
    SuggestWord(String, Suggestion),
    UnknownSubcommand(Arg<'a>, Subcommand),
    BrokenRule(ParseError),
    Help,
}

#[derive(Debug, PartialEq, Copy, Clone)]
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
    ExceedingMaxCount,
}

impl<'a> std::error::Error for Error<'a> {}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {

        match self.context() {
            ErrorContext::ExceededThreshold(arg, cur, max) => {
                write!(f, "option '{}' can be used up to {} times but was supplied {} times", arg.to_string(), max, cur)
            },
            ErrorContext::Help => {
                write!(f, "{}", self.help.as_ref().unwrap_or(&Help::new()).get_quick_text())
            },
            ErrorContext::FailedCast(arg, val, err) => {
                write!(f, "argument '{}' failed to process '{}' due to: {}", arg.to_string(), val, err)
            },
            ErrorContext::FailedArg(arg) => {
                match self.kind() {
                    ErrorKind::MissingPositional => {
                        let usage = match self.help.as_ref().unwrap_or(&Help::new()).get_usage() { 
                            Some(m) => { "\n\n".to_owned() + m } 
                            None => { "".to_owned() } 
                        };
                        write!(f, "missing positional argument '{}'{}", arg.to_string(), usage)
                    },
                    ErrorKind::DuplicateOptions => {
                        write!(f, "option '{}' can only be supplied 1 time", arg.to_string())
                    },
                    ErrorKind::ExpectingValue => {
                        write!(f, "option '{}' take 1 value but 0 was supplied", arg.to_string())
                    },
                    _ => panic!("reached unreachable error kind for a failed argument error context"),
                }
            },
            ErrorContext::SuggestWord(word, suggestion) => {
                match self.kind() {
                    ErrorKind::SuggestArg => {
                        write!(f, "unknown argument '{}'\n\nDid you mean '{}'?", word, suggestion)
                    },
                    ErrorKind::SuggestSubcommand => {
                        write!(f, "unknown subcommand '{}'\n\nDid you mean '{}'?", word, suggestion)
                    },
                    _ => panic!("reached unreachable error kind for a failed argument error context"),
                }
            },
            ErrorContext::OutofContextArgSuggest(arg, subcommand) => {
                write!(f, "argument '{}' is unknown or invalid in the current context\n\nMaybe move it after '{}'?", arg, subcommand)
            },
            ErrorContext::UnexpectedValue(flag, val) => {
                write!(f, "flag '{}' cannot accept a value but was given '{}'", flag.to_string(), val)
            },
            ErrorContext::UnexpectedArg(word) => {
                write!(f, "unknown argument '{}'", word)
            },
            ErrorContext::UnknownSubcommand(arg, subcommand) => {
                write!(f, "unknown subcommand '{}' for '{}'", subcommand, arg.to_string())
            },
            ErrorContext::BrokenRule(err) => {
                write!(f, "a rule conflict occurred due to: {}", err)
            },
        }?;
        Ok(())
    }
}
