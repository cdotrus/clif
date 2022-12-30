#[cfg(feature = "color")]
use crayon::*;

use crate::arg::Arg;
use crate::help::Help;
use std::fmt::Display;

const NEW_PARAGRAPH: &str = "\n\n";

mod exit_code {
    pub const BAD: u8 = 101;
    pub const OKAY: u8 = 0;
}

type Value = String;
type Subcommand = String;
type Suggestion = String;
type MaxCount = usize;
type CurCount = usize;
type SomeError = Box<dyn std::error::Error>;
type Argument = String;

#[derive(Debug)]
pub struct Error<'a> {
    #[cfg(feature = "color")]
    use_color: bool,
    context: ErrorContext<'a>,
    help: Option<Help<'a>>,
    kind: ErrorKind,
}

impl<'a> Error<'a> {
    /// Creates a new error.
    pub fn new(
        help: Option<Help<'a>>,
        kind: ErrorKind,
        context: ErrorContext<'a>,
        _use_color: bool,
    ) -> Self {
        Self {
            #[cfg(feature = "color")]
            use_color: _use_color,
            help: help,
            kind: kind,
            context: context,
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
    fn help_tip(&self) -> Option<String> {
        let flag_str = self.help.as_ref()?.get_flag().to_string();
        #[cfg(feature = "color")]
        let flag_str = match self.use_color {
            true => flag_str.green().to_string(),
            false => flag_str,
        };
        Some(format!(
            "{}For more information, try '{}'.",
            NEW_PARAGRAPH, flag_str
        ))
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorContext<'a> {
    ExceededThreshold(Arg<'a>, CurCount, MaxCount),
    FailedArg(Arg<'a>),
    UnexpectedValue(Arg<'a>, Value),
    FailedCast(Arg<'a>, Value, SomeError),
    OutofContextArgSuggest(Argument, Subcommand),
    UnexpectedArg(Argument),
    SuggestWord(String, Suggestion),
    UnknownSubcommand(Arg<'a>, Subcommand),
    CustomRule(SomeError),
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
    CustomRule,
    Help,
    ExceedingMaxCount,
}

impl<'a> std::error::Error for Error<'a> {}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        #[cfg(feature = "color")]
        let color = |a: ColoredString| -> String {
            match self.use_color {
                true => a.to_string(),
                false => a.get_data().to_string(),
            }
        };

        match self.context() {
            ErrorContext::ExceededThreshold(arg, cur, max) => {
                let arg_str = arg.to_string();
                #[cfg(feature = "color")]
                let arg_str = color(arg_str.blue());
                write!(
                    f,
                    "option '{}' can be used up to {} times but was supplied {} times",
                    arg_str, max, cur
                )
            }
            ErrorContext::Help => {
                write!(
                    f,
                    "{}",
                    self.help.as_ref().unwrap_or(&Help::new()).get_quick_text()
                )
            }
            ErrorContext::FailedCast(arg, val, err) => {
                let arg_str = arg.to_string();
                #[cfg(feature = "color")]
                let arg_str = color(arg_str.blue());
                let val_str = val.to_string();
                #[cfg(feature = "color")]
                let val_str = color(val_str.yellow());
                write!(
                    f,
                    "argument '{}' failed to process '{}' due to: {}",
                    arg_str, val_str, err
                )
            }
            ErrorContext::FailedArg(arg) => match self.kind() {
                ErrorKind::MissingPositional => {
                    let usage = match self.help.as_ref().unwrap_or(&Help::new()).get_usage() {
                        Some(m) => NEW_PARAGRAPH.to_owned() + m,
                        None => "".to_owned(),
                    };
                    let arg_str = arg.to_string();
                    #[cfg(feature = "color")]
                    let arg_str = color(arg_str.blue());
                    write!(f, "missing positional argument '{}'{}", arg_str, usage)
                }
                ErrorKind::DuplicateOptions => {
                    let arg_str = arg.to_string();
                    #[cfg(feature = "color")]
                    let arg_str = color(arg_str.blue());
                    write!(f, "argument '{}' can only be supplied once", arg_str)
                }
                ErrorKind::ExpectingValue => {
                    let arg_str = arg.to_string();
                    #[cfg(feature = "color")]
                    let arg_str = color(arg_str.blue());
                    write!(f, "option '{}' takes 1 value but 0 was supplied", arg_str)
                }
                _ => panic!("reached unreachable error kind for a failed argument error context"),
            },
            ErrorContext::SuggestWord(word, suggestion) => match self.kind() {
                ErrorKind::SuggestArg => {
                    #[cfg(feature = "color")]
                    let word = color(word.yellow());
                    #[cfg(feature = "color")]
                    let suggestion = color(suggestion.green());
                    write!(
                        f,
                        "invalid argument '{}'{}Did you mean '{}'?",
                        word, NEW_PARAGRAPH, suggestion
                    )
                }
                ErrorKind::SuggestSubcommand => {
                    #[cfg(feature = "color")]
                    let word = color(word.yellow());
                    #[cfg(feature = "color")]
                    let suggestion = color(suggestion.green());
                    write!(
                        f,
                        "invalid subcommand '{}'{}Did you mean '{}'?",
                        word, NEW_PARAGRAPH, suggestion
                    )
                }
                _ => panic!("reached unreachable error kind for a failed argument error context"),
            },
            ErrorContext::OutofContextArgSuggest(arg, subcommand) => {
                let arg_str = arg.to_string();
                #[cfg(feature = "color")]
                let arg_str = color(arg_str.yellow());
                #[cfg(feature = "color")]
                let subcommand = color(subcommand.green());
                write!(f, "argument '{}' is unknown or invalid in the current context{}Maybe move it after '{}'?", arg_str, NEW_PARAGRAPH, subcommand)
            }
            ErrorContext::UnexpectedValue(flag, val) => {
                let flag_str = flag.to_string();
                #[cfg(feature = "color")]
                let flag_str = color(flag_str.blue());
                #[cfg(feature = "color")]
                let val_str = color(val.yellow());
                write!(
                    f,
                    "flag '{}' cannot accept a value but was given '{}'",
                    flag_str, val_str
                )
            }
            ErrorContext::UnexpectedArg(word) => {
                #[cfg(feature = "color")]
                let word = color(word.yellow());
                write!(
                    f,
                    "invalid argument '{}'{}",
                    word,
                    self.help_tip().unwrap_or(String::new())
                )
            }
            ErrorContext::UnknownSubcommand(arg, subcommand) => {
                #[cfg(feature = "color")]
                let subcommand = color(subcommand.yellow());
                #[cfg(feature = "color")]
                let arg_str = color(arg.to_string().blue());
                write!(f, "invalid subcommand '{}' for '{}'", subcommand, arg_str)
            }
            ErrorContext::CustomRule(err) => {
                write!(f, "{}", err)
            }
        }?;
        Ok(())
    }
}
