use crate::arg::Arg;
use std::fmt::Display;
use std::error::Error;
use colored::*;
use crate::cli::Help;

#[derive(Debug, PartialEq)]
pub enum CliError<'a> {
    BadType(Arg<'a>, String, String),
    MissingPositional(Arg<'a>, Help<'a>),
    DuplicateOptions(Arg<'a>),
    ExpectingValue(Arg<'a>),
    UnexpectedValue(Arg<'a>, String),
    OutOfContextArgSuggest(String, String),
    UnexpectedArg(String),
    SuggestArg(String, String),
    SuggestSubcommand(String, String),
    UnknownSubcommand(Arg<'a>, String),
    BrokenRule(String),
    Help(&'a str),
}

impl<'a> CliError<'a> {
    pub fn code(&self) -> u8 {
        match &self {
            Self::Help(_) => 0,
            _ => 101
        }
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
                        if let Some(text) = help.info().split_terminator('\n').skip(i).next() {
                            "\n\nUsage:\n".to_string() + text
                        } else {
                            panic!("usage directive line is out of bounds on help:\n\"\"\"\n{}\"\"\"", help.info())
                        }
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