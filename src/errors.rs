use crate::arg::Arg;
use std::fmt::Display;
use std::error::Error;
use colored::*;
use crate::cli::Help;

const BAD_CODE: u8 = 101;
const OKAY_CODE: u8 = 0;

#[derive(Debug, PartialEq)]
pub enum CliError<'a> {
    /// argument, value, error message 
    BadType(Arg<'a>, String, String),
    /// argument, help
    MissingPositional(Arg<'a>, Help<'a>),
    /// argument
    DuplicateOptions(Arg<'a>),
    /// argument
    ExpectingValue(Arg<'a>),
    /// argument, value
    UnexpectedValue(Arg<'a>, String),
    /// argument, subcommand
    OutOfContextArgSuggest(String, String),
    /// argument
    UnexpectedArg(String),
    /// argument, suggestion
    SuggestArg(String, String),
    /// subcommand, suggestion
    SuggestSubcommand(String, String),
    /// argument, previous command
    UnknownSubcommand(Arg<'a>, String),
    /// error message
    BrokenRule(String),
    /// quick help text
    Help(&'a str),
    /// n, incorrect size, argument
    ExceedingMaxCount(usize, usize, Arg<'a>),
}

impl<'a> CliError<'a> {
    /// Returns `OKAY_CODE` for help error and `BAD_CODE` otherwise.
    pub fn code(&self) -> u8 {
        match &self {
            Self::Help(_) => OKAY_CODE,
            _ => BAD_CODE
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
            Self::Help(h) => write!(f, "{}", h),
            Self::SuggestArg(a, sug) => write!(f, "unknown argument '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::SuggestSubcommand(a, sug) => write!(f, "unknown subcommand '{}'\n\nDid you mean {}?", a.yellow(), sug.green()),
            Self::OutOfContextArgSuggest(o, cmd) => write!(f, "argument '{}' is unknown, or invalid in the current context\n\nMaybe move it after '{}'?", o.yellow(), cmd.green()),
            Self::BadType(a, v, e) => write!(f, "argument '{}' did not process '{}' due to {}", a.to_string().yellow(), v.blue(), e),
            Self::MissingPositional(p, help) => {
                // detect the usage statement to print from command's short help text
                let usage = match help.usage_at() { 
                    None => String::new(),
                    Some(i) =>  {
                        let mut content = String::from("\n");
                        let mut lines = help.info().split_terminator('\n').skip(i.start);
                        for _ in 0..i.end-i.start {
                            content.push('\n');
                            content.push_str(lines.next().expect("usage directive range is out of bounds"));
                        }
                        content
                    }
                };
                write!(f, "missing required argument '{}'{}", p, usage)
            },
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