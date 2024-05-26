use crate::arg::ArgType;
use crate::help::Help;
use colored::Colorize;
use std::fmt::Display;

#[derive(Debug, PartialEq, Clone)]
pub enum ColorMode {
    On,
    Off,
    Normal,
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Normal
    }
}

impl ColorMode {
    pub fn new() -> Self {
        Self::Off
    }

    pub fn sync(&self) {
        match self {
            Self::On => colored::control::set_override(true),
            Self::Off => colored::control::set_override(false),
            Self::Normal => colored::control::unset_override(),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CapMode {
    Upper,
    Lower,
    Manual,
}

impl CapMode {
    pub fn new() -> Self {
        Self::Manual
    }
}

impl Default for CapMode {
    fn default() -> Self {
        Self::Lower
    }
}

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
pub struct Error {
    context: ErrorContext,
    cap_mode: CapMode,
    help: Option<Help>,
    kind: ErrorKind,
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::new(
            None,
            ErrorKind::CustomRule,
            ErrorContext::CustomRule(value),
            CapMode::default(),
        )
    }
}

impl Error {
    /// Creates a new error.
    pub fn new(
        help: Option<Help>,
        kind: ErrorKind,
        context: ErrorContext,
        cap_mode: CapMode,
    ) -> Self {
        Self {
            help: help,
            kind: kind,
            context: context,
            cap_mode: cap_mode,
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
        let flag_str = crate::arg::into_data(self.help.as_ref()?.get_arg()).to_string();
        Some(format!(
            "{}For more information, try \"{}\".",
            NEW_PARAGRAPH,
            flag_str.green()
        ))
    }

    /// Transforms any error into a custom rule error to be used during [crate::Cli] parsing.
    pub fn validate<U, E: std::error::Error + 'static>(rule: Result<U, E>) -> Result<U, Self> {
        match rule {
            Ok(t) => Ok(t),
            Err(e) => Err(Self::new(
                None,
                ErrorKind::CustomRule,
                ErrorContext::CustomRule(Box::new(e)),
                CapMode::default(),
            )),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorContext {
    ExceededThreshold(ArgType, CurCount, MaxCount),
    FailedArg(ArgType),
    UnexpectedValue(ArgType, Value),
    FailedCast(ArgType, Value, SomeError),
    OutofContextArgSuggest(Argument, Subcommand),
    UnexpectedArg(Argument),
    SuggestWord(String, Suggestion),
    UnknownSubcommand(ArgType, Subcommand),
    CustomRule(SomeError),
    Help,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ErrorKind {
    BadType,
    MissingPositional,
    MissingOption,
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

impl std::error::Error for Error {}

/// Capitalizes a sentence by converting the first character to uppercase (if possible).
pub fn capitalize(s: String) -> String {
    s.char_indices()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect()
}

/// De-capitalizes a sentence by converting the first character to uppercase (if possible).
pub fn lowerize(s: String) -> String {
    s.char_indices()
        .map(|(i, c)| if i == 0 { c.to_ascii_lowercase() } else { c })
        .collect()
}

pub fn format_err_msg(s: String, cap_mode: CapMode) -> String {
    match cap_mode {
        CapMode::Upper => capitalize(s),
        CapMode::Lower => lowerize(s),
        CapMode::Manual => s,
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self.context() {
            ErrorContext::ExceededThreshold(arg, cur, max) => {
                write!(
                    f,
                    "option \"{}\" can be used up to {} times but was supplied {} times",
                    arg.to_string().blue(),
                    max,
                    cur
                )
            }
            ErrorContext::Help => {
                write!(
                    f,
                    "{}",
                    self.help.as_ref().unwrap_or(&Help::new()).get_text()
                )
            }
            ErrorContext::FailedCast(arg, val, err) => {
                write!(
                    f,
                    "argument \"{}\" failed to process value \"{}\": {}",
                    arg.to_string().blue(),
                    val.to_string().yellow(),
                    format_err_msg(err.to_string(), self.cap_mode)
                )
            }
            ErrorContext::FailedArg(arg) => match self.kind() {
                ErrorKind::MissingPositional => {
                    write!(
                        f,
                        "missing positional argument \"{}\"{}",
                        arg.to_string().blue(),
                        self.help_tip().unwrap_or(String::new())
                    )
                }
                ErrorKind::MissingOption => {
                    write!(
                        f,
                        "missing required option \"{}\"{}",
                        arg.to_string().blue(),
                        self.help_tip().unwrap_or(String::new())
                    )
                }
                ErrorKind::DuplicateOptions => {
                    write!(
                        f,
                        "argument \"{}\" can only be supplied once",
                        arg.to_string().blue()
                    )
                }
                ErrorKind::ExpectingValue => {
                    write!(
                        f,
                        "option \"{}\" accepts one value but zero were supplied",
                        arg.to_string().blue()
                    )
                }
                _ => panic!("reached unreachable error kind for a failed argument error context"),
            },
            ErrorContext::SuggestWord(word, suggestion) => match self.kind() {
                ErrorKind::SuggestArg => {
                    write!(
                        f,
                        "invalid argument \"{}\"{}Did you mean \"{}\"?",
                        word.yellow(),
                        NEW_PARAGRAPH,
                        suggestion.green()
                    )
                }
                ErrorKind::SuggestSubcommand => {
                    write!(
                        f,
                        "invalid subcommand \"{}\"{}Did you mean \"{}\"?",
                        word.yellow(),
                        NEW_PARAGRAPH,
                        suggestion.green()
                    )
                }
                _ => panic!("reached unreachable error kind for a failed argument error context"),
            },
            ErrorContext::OutofContextArgSuggest(arg, subcommand) => {
                write!(f, "argument \"{}\" is unknown or invalid in the current context{}Maybe move it after \"{}\"?", arg.yellow(), NEW_PARAGRAPH, subcommand.green())
            }
            ErrorContext::UnexpectedValue(flag, val) => {
                write!(
                    f,
                    "flag \"{}\" cannot accept a value but was given \"{}\"",
                    flag.to_string().blue(),
                    val.yellow()
                )
            }
            ErrorContext::UnexpectedArg(word) => {
                write!(
                    f,
                    "invalid argument \"{}\"{}",
                    word.yellow(),
                    self.help_tip().unwrap_or(String::new())
                )
            }
            ErrorContext::UnknownSubcommand(arg, subcommand) => {
                write!(
                    f,
                    "invalid subcommand \"{}\" for \"{}\"",
                    subcommand.yellow(),
                    arg.to_string().blue()
                )
            }
            ErrorContext::CustomRule(err) => {
                write!(f, "{}", format_err_msg(err.to_string(), self.cap_mode))
            }
        }?;
        Ok(())
    }
}
