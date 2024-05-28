use crate::error::{self, CapMode, ColorMode};
use crate::help::Help;
use crate::seqalin;
use crate::seqalin::Cost;
use crate::{arg::*, Command, Subcommand};
use colored::Colorize;
use stage::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::process::ExitCode;
use std::str::FromStr;

pub use crate::error::{Error, ErrorContext, ErrorKind};

/// The return type for a [Command]'s interpretation process.
pub type Result<T> = std::result::Result<T, Error>;

mod symbol {
    // series of characters to denote flags and switches
    pub const SWITCH: &str = "-";
    // @note: tokenizing depends on flag having the first character be the switch character
    pub const FLAG: &str = "--";
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum Tag<T: AsRef<str>> {
    Switch(T),
    Flag(T),
}

impl<T: AsRef<str>> Tag<T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Flag(s) => s,
            Self::Switch(s) => s,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    UnattachedArgument(usize, String),
    AttachedArgument(usize, String),
    Flag(usize),
    Switch(usize, char),
    EmptySwitch(usize),
    Ignore(usize, String),
    Terminator(usize),
}

impl Token {
    fn take_str(self) -> String {
        match self {
            Self::UnattachedArgument(_, s) => s,
            Self::AttachedArgument(_, s) => s,
            Self::Ignore(_, s) => s,
            _ => panic!("cannot call take_str on token without string"),
        }
    }

    fn _get_index_ref(&self) -> &usize {
        match self {
            Self::UnattachedArgument(i, _) => i,
            Self::AttachedArgument(i, _) => i,
            Self::Flag(i) => i,
            Self::EmptySwitch(i) => i,
            Self::Switch(i, _) => i,
            Self::Terminator(i) => i,
            Self::Ignore(i, _) => i,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Slot {
    pointers: Vec<usize>,
    visited: bool,
}

impl Slot {
    fn new() -> Self {
        Self {
            pointers: Vec::new(),
            visited: false,
        }
    }

    fn push(&mut self, i: usize) -> () {
        self.pointers.push(i);
    }

    fn is_visited(&self) -> bool {
        self.visited
    }

    fn visit(&mut self) -> () {
        self.visited = true;
    }

    fn get_indices(&self) -> &Vec<usize> {
        &self.pointers
    }

    fn first(&self) -> Option<&usize> {
        self.pointers.first()
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum MemoryState {
    Start,
    ProcessingFlags,
    ProcessingOptionals,
    ProcessingPositionals,
    ProcessingSubcommands,
    End,
}

impl MemoryState {
    /// Ensures the previous state (`self`) can transition to the next state (`next`).
    pub fn proceed(&mut self, mut next: MemoryState) {
        // panic if we are already advanced past the next state
        if self > &mut next {
            panic!("{}: argument discovery is in an invalid order: invalid state transition from {:?} to {:?}", "panic!".red().bold().underline(), self, next)
        }
        // println!("{:?} -> {:?}", self, next);
        *self = next;
    }

    pub fn reset() -> Self {
        Self::Start
    }
}

pub mod stage {
    /// The typestate pattern for the different stages in processing data from 
    /// the command-line.
    pub trait ProcessorState {}

    /// The first stage in the command-line processor. The processor can be
    /// configured when it is in this state. 
    pub struct Build;

    /// The second stage in the command-line processor. The processor can be
    /// routed to different modes of processing.
    pub struct Ready;

    /// The third and final stage in the command-line processor. The processor
    /// stores the command-line data and allows for requests to query what data
    /// it captured.
    pub struct Memory;

    impl ProcessorState for Build {}

    impl ProcessorState for Ready {}

    impl ProcessorState for Memory {}
}

impl<S: ProcessorState> Cli<S> {
    /// Perform a state transition for the command-line processor.
    fn transition<T: ProcessorState>(self) -> Cli<T> {
        Cli::<T> {
            tokens: self.tokens,
            store: self.store,
            known_args: self.known_args,
            asking_for_help: self.asking_for_help,
            help: self.help,
            state: self.state,
            options: self.options,
            _marker: PhantomData::<T>,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct CliOptions {
    pub prioritize_help: bool,
    pub cap_mode: CapMode,
    pub threshold: Cost,
    pub capacity: usize,
    pub color_mode: ColorMode,
    pub err_prefix: String,
    pub err_suffix: String,
}

impl CliOptions {
    pub fn new() -> Self {
        Self {
            prioritize_help: true,
            cap_mode: CapMode::new(),
            threshold: 0,
            capacity: 0,
            color_mode: ColorMode::new(),
            err_prefix: String::new(),
            err_suffix: String::new(),
        }
    }
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            prioritize_help: true,
            cap_mode: CapMode::default(),
            threshold: 2,
            capacity: 0,
            color_mode: ColorMode::default(),
            err_prefix: String::from(format!("{}: ", "error".red().bold())),
            err_suffix: String::new(),
        }
    }
}

/// The command-line processor.
#[derive(Debug, PartialEq)]
pub struct Cli<S: ProcessorState> {
    /// The order-preserved list of tokens
    tokens: Vec<Option<Token>>,
    /// A lookup table for identifying which positions in the token stream a given option is present
    store: HashMap<Tag<String>, Slot>,
    /// The list of arguments has they are processed by the Cli processor
    known_args: Vec<ArgType>,
    asking_for_help: bool,
    help: Option<Help>,
    state: MemoryState,
    options: CliOptions,
    _marker: PhantomData<S>,
}

impl Default for Cli<Build> {
    fn default() -> Self {
        Self {
            tokens: Vec::default(),
            store: HashMap::default(),
            known_args: Vec::default(),
            help: None,
            asking_for_help: false,
            state: MemoryState::Start,
            options: CliOptions::default(),
            _marker: PhantomData,
        }
    }
}

impl Cli<Build> {
    /// Create a new command-line processor. The processor is configured with
    /// minimal options enabled.
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            store: HashMap::new(),
            known_args: Vec::new(),
            help: None,
            asking_for_help: false,
            state: MemoryState::Start,
            options: CliOptions::new(),
            _marker: PhantomData,
        }
    }

    /// Sets the initial capacity for the data structures that are used to hold
    /// the processed command-line data.
    pub fn with_capacity(mut self, cap: usize) -> Self {
        self.options.capacity = cap;
        self
    }

    /// Sets the maximum threshold value when comparing strings for character similiarity.
    pub fn threshold(mut self, cost: Cost) -> Self {
        self.options.threshold = cost;
        self
    }

    /// Automatically uppercase error messages during program execution.
    pub fn auto_uppercase_errors(mut self) -> Self {
        self.options.cap_mode = CapMode::Upper;
        self
    }

    /// Automatically lowercase error messages during program execution.
    pub fn auto_lowercase_errors(mut self) -> Self {
        self.options.cap_mode = CapMode::Lower;
        self
    }

    /// Do not allow any formatting on error messages during program execution.
    pub fn disable_auto_case_errors(mut self) -> Self {
        self.options.cap_mode = CapMode::Manual;
        self
    }

    /// Enables coloring for the output.
    pub fn enable_color(mut self) -> Self {
        self.options.color_mode = ColorMode::On;
        self
    }

    /// Disables coloring for the output.
    pub fn disable_color(mut self) -> Self {
        self.options.color_mode = ColorMode::Off;
        self
    }

    /// Allows the output to be colored, but determines coloring based on
    /// other factors in the environment.
    pub fn allow_color(mut self) -> Self {
        self.options.color_mode = ColorMode::Normal;
        self
    }

    /// Downplays the [Help] flag to not become a priority error over other errors
    /// during interpretation.
    ///
    /// Help is prioritized by default.
    pub fn deprioritize_help(mut self) -> Self {
        self.options.prioritize_help = false;
        self
    }

    /// Prioritizes the [Help] flag over other errors during interpretation.
    ///
    /// This is enabled by default.
    pub fn prioritize_help(mut self) -> Self {
        self.options.prioritize_help = true;
        self
    }

    /// Sets the text to come before an error message if one is reported during
    /// processing.
    pub fn error_prefix<T: AsRef<str>>(mut self, prefix: T) -> Self {
        self.options.err_prefix = String::from(prefix.as_ref());
        self
    }

    /// Sets the text to come after an error message if one is reported during
    /// processing.
    pub fn error_suffix<T: AsRef<str>>(mut self, suffix: T) -> Self {
        self.options.err_suffix = String::from(suffix.as_ref());
        self
    }

    /// Builds the [Cli] struct by tokenizing the [String] iterator into a
    /// representable form for further processing.
    ///
    /// This function transitions the [Cli] state to the [Ready] state.
    pub fn parse<T: Iterator<Item = String>>(mut self, args: T) -> Cli<Ready> {
        self.options.color_mode.sync();
        let mut tokens = Vec::<Option<Token>>::with_capacity(self.options.capacity);
        let mut store = HashMap::with_capacity(self.options.capacity);
        let mut terminated = false;
        let mut args = args.skip(1).enumerate();
        while let Some((i, mut arg)) = args.next() {
            // ignore all input after detecting the terminator
            if terminated == true {
                tokens.push(Some(Token::Ignore(i, arg)));
            // handle an option
            } else if arg.starts_with(symbol::SWITCH) == true {
                // try to separate from '=' sign
                let mut value: Option<String> = None;
                let mut option: Option<String> = None;
                {
                    if let Some((opt, val)) = arg.split_once('=') {
                        option = Some(opt.to_string());
                        value = Some(val.to_string());
                    }
                }
                // update arg to be the value split by '='
                if let Some(opt) = option {
                    arg = opt;
                }
                // handle long flag signal
                if arg.starts_with(symbol::FLAG) == true {
                    arg.replace_range(0..=1, "");
                    // caught the terminator (purely "--")
                    if arg.is_empty() == true {
                        tokens.push(Some(Token::Terminator(i)));
                        terminated = true;
                    // caught a 'long option' flag
                    } else {
                        store
                            .entry(Tag::Flag(arg))
                            .or_insert(Slot::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::Flag(i)));
                    }
                // handle short flag signal
                } else {
                    // skip the initial switch character/symbol (1 char)
                    let mut arg = arg.chars().skip(1);
                    // check if the switch is empty by evaulating the first possible switch position
                    if let Some(c) = arg.next() {
                        store
                            .entry(Tag::Switch(c.to_string()))
                            .or_insert(Slot::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::Switch(i, c)));
                    } else {
                        store
                            .entry(Tag::Switch(String::new()))
                            .or_insert(Slot::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::EmptySwitch(i)));
                    }
                    // continuously split switches into individual components
                    while let Some(c) = arg.next() {
                        store
                            .entry(Tag::Switch(c.to_string()))
                            .or_insert(Slot::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::Switch(i, c)));
                    }
                }
                // caught an argument directly attached to an option
                if let Some(val) = value {
                    tokens.push(Some(Token::AttachedArgument(i, val)));
                }
            // caught an argument
            } else {
                tokens.push(Some(Token::UnattachedArgument(i, arg)));
            }
        }
        self.tokens = tokens;
        self.store = store;
        // proceed to the next state
        Cli::transition(self)
    }
}

impl Cli<Ready> {
    /// Runs the remaining steps in the command-line processor.
    ///
    /// This function transitions the [Cli] processor to the [Memory] state, and
    /// then calls the necessary methods for `T` to behave as an implementation
    /// of the [Command] trait.
    ///
    /// 1. `T` interprets the command-line data into its own structural data
    /// 2. `T` executes its task
    ///
    /// This function will handle errors and report them to `stderr` if one
    /// is encountered. If an error is encountered, the function returns 101 as 
    /// the exit code. If no error is encountered, the function returns 0 as the 
    /// exit code.
    pub fn go<T: Command>(self) -> ExitCode {
        let mut cli: Cli<Memory> = self.save();

        match T::interpret(&mut cli) {
            // construct the application
            Ok(program) => {
                // verify the cli has no additional arguments if this is the top-level command being parsed
                match cli.is_empty() {
                    Ok(_) => {
                        let cli_opts = cli.options.clone();
                        std::mem::drop(cli);
                        match program.execute() {
                            Ok(_) => ExitCode::from(0),
                            Err(err) => {
                                eprintln!(
                                    "{}{}{}",
                                    cli_opts.err_prefix,
                                    error::format_err_msg(err.to_string(), cli_opts.cap_mode),
                                    cli_opts.err_suffix
                                );
                                ExitCode::from(101)
                            }
                        }
                    }
                    // report cli error
                    Err(err) => {
                        let cli_opts = cli.options;
                        match err.kind() {
                            ErrorKind::Help => println!("{}", &err),
                            _ => eprintln!(
                                "{}{}{}",
                                cli_opts.err_prefix,
                                error::format_err_msg(err.to_string(), cli_opts.cap_mode),
                                cli_opts.err_suffix
                            ),
                        }
                        ExitCode::from(err.code())
                    }
                }
            }
            // report cli error
            Err(err) => {
                let cli_opts = cli.options;
                match err.kind() {
                    ErrorKind::Help => println!("{}", &err),
                    _ => eprintln!(
                        "{}{}{}",
                        cli_opts.err_prefix,
                        error::format_err_msg(err.to_string(), cli_opts.cap_mode),
                        cli_opts.err_suffix
                    ),
                }
                ExitCode::from(err.code())
            }
        }
    }

    /// Saves the data from the command-line processing to be recalled during
    /// interpretation.
    pub fn save(self) -> Cli<Memory> {
        self.transition()
    }
}

// Private API

impl Cli<Memory> {
    /// Serves the next `Positional` value in the token stream parsed as `T`.
    ///
    /// Errors if parsing fails. If the next argument is not a positional, it will
    /// not move forward in the token stream.
    fn get_positional<'a, T: FromStr>(&mut self, p: Positional) -> Result<Option<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        self.known_args.push(ArgType::Positional(p));
        self.try_positional()
    }

    fn get_positional_all<'a, T: FromStr>(&mut self, p: Positional) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        let mut result = Vec::<T>::new();
        match self.get_positional(p)? {
            Some(item) => result.push(item),
            None => return Ok(None),
        }
        while let Some(v) = self.try_positional()? {
            result.push(v);
        }
        Ok(Some(result))
    }

    fn get_positional_until<'a, T: FromStr>(
        &mut self,
        p: Positional,
        limit: usize,
    ) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        let values = self.get_positional_all::<T>(p)?;
        match values {
            // verify the size of the vector does not exceed `n`
            Some(r) => match r.len() <= limit {
                true => Ok(Some(r)),
                false => Err(Error::new(
                    self.help.clone(),
                    ErrorKind::ExceedingMaxCount,
                    ErrorContext::ExceededThreshold(self.known_args.pop().unwrap(), r.len(), limit),
                    self.options.cap_mode,
                )),
            },
            None => Ok(None),
        }
    }

    /// Forces the next [Positional] to exist from token stream.
    ///
    /// Errors if parsing fails or if no unattached argument is left in the token stream.
    fn require_positional<'a, T: FromStr>(&mut self, p: Positional) -> Result<T>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        if let Some(value) = self.get_positional(p)? {
            Ok(value)
        } else {
            self.try_to_help()?;
            self.is_empty()?;
            Err(Error::new(
                self.help.clone(),
                ErrorKind::MissingPositional,
                ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                self.options.cap_mode,
            ))
        }
    }

    /// Forces all the next [Positional] to be captured from the token stream.
    ///
    /// Errors if parsing fails or if zero unattached arguments are left in the token stream to begin.
    ///
    /// The resulting vector is guaranteed to have `.len() >= 1`.
    fn require_positional_all<'a, T: FromStr>(&mut self, p: Positional) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        let mut result = Vec::<T>::new();
        result.push(self.require_positional(p)?);
        while let Some(v) = self.try_positional()? {
            result.push(v);
        }
        Ok(result)
    }

    fn require_positional_until<'a, T: FromStr>(
        &mut self,
        p: Positional,
        limit: usize,
    ) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingPositionals);
        let values = self.require_positional_all(p)?;
        // verify the size of the vector does not exceed `n`
        match values.len() <= limit {
            true => Ok(values),
            false => Err(Error::new(
                self.help.clone(),
                ErrorKind::ExceedingMaxCount,
                ErrorContext::ExceededThreshold(
                    self.known_args.pop().unwrap(),
                    values.len(),
                    limit,
                ),
                self.options.cap_mode,
            )),
        }
    }

    /// Queries for a value of `Optional`.
    ///
    /// Errors if there are multiple values or if parsing fails.
    fn get_option<'a, T: FromStr>(&mut self, o: Optional) -> Result<Option<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(o.get_flag().get_name());
        if let Some(c) = o.get_flag().get_switch() {
            locs.extend(self.take_switch_locs(c));
        }
        self.known_args.push(ArgType::Optional(o));
        // pull values from where the option flags were found (including switch)
        let mut values = self.pull_flag(locs, true);
        match values.len() {
            1 => {
                if let Some(word) = values.pop().unwrap() {
                    let result = word.parse::<T>();
                    match result {
                        Ok(r) => Ok(Some(r)),
                        Err(err) => {
                            self.try_to_help()?;
                            Err(Error::new(
                                self.help.clone(),
                                ErrorKind::BadType,
                                ErrorContext::FailedCast(
                                    self.known_args.pop().unwrap(),
                                    word,
                                    Box::new(err),
                                ),
                                self.options.cap_mode,
                            ))
                        }
                    }
                } else {
                    self.try_to_help()?;
                    Err(Error::new(
                        self.help.clone(),
                        ErrorKind::ExpectingValue,
                        ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                        self.options.cap_mode,
                    ))
                }
            }
            0 => Ok(None),
            _ => {
                self.try_to_help()?;
                Err(Error::new(
                    self.help.clone(),
                    ErrorKind::DuplicateOptions,
                    ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                    self.options.cap_mode,
                ))
            }
        }
    }

    /// Queries for all values behind an `Optional`.
    ///
    /// Errors if a parsing fails from string.
    fn get_option_all<'a, T: FromStr>(&mut self, o: Optional) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(o.get_flag().get_name());
        if let Some(c) = o.get_flag().get_switch() {
            locs.extend(self.take_switch_locs(c));
        }
        self.known_args.push(ArgType::Optional(o));
        // pull values from where the option flags were found (including switch)
        let values = self.pull_flag(locs, true);
        if values.is_empty() == true {
            return Ok(None);
        }
        // try to convert each value into the type T
        let mut transform = Vec::<T>::with_capacity(values.len());
        for val in values {
            if let Some(word) = val {
                let result = word.parse::<T>();
                match result {
                    Ok(r) => transform.push(r),
                    Err(err) => {
                        self.try_to_help()?;
                        return Err(Error::new(
                            self.help.clone(),
                            ErrorKind::BadType,
                            ErrorContext::FailedCast(
                                self.known_args.pop().unwrap(),
                                word,
                                Box::new(err),
                            ),
                            self.options.cap_mode,
                        ));
                    }
                }
            } else {
                self.try_to_help()?;
                return Err(Error::new(
                    self.help.clone(),
                    ErrorKind::ExpectingValue,
                    ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                    self.options.cap_mode,
                ));
            }
        }
        Ok(Some(transform))
    }

    /// Queries for up to `n` values behind an `Optional`.
    ///
    /// Errors if a parsing fails from string or if the number of detected optionals is > n.
    fn get_option_until<'a, T: FromStr>(
        &mut self,
        o: Optional,
        limit: usize,
    ) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        let values = self.get_option_all::<T>(o)?;
        match values {
            // verify the size of the vector does not exceed `n`
            Some(r) => match r.len() <= limit {
                true => Ok(Some(r)),
                false => Err(Error::new(
                    self.help.clone(),
                    ErrorKind::ExceedingMaxCount,
                    ErrorContext::ExceededThreshold(self.known_args.pop().unwrap(), r.len(), limit),
                    self.options.cap_mode,
                )),
            },
            None => Ok(None),
        }
    }

    /// Queries for an expected value of `Optional`.
    fn require_option<'a, T: FromStr>(&mut self, o: Optional) -> Result<T>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        if let Some(value) = self.get_option(o)? {
            Ok(value)
        } else {
            self.try_to_help()?;
            self.is_empty()?;
            Err(Error::new(
                self.help.clone(),
                ErrorKind::MissingOption,
                ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                self.options.cap_mode,
            ))
        }
    }

    fn require_option_all<'a, T: FromStr>(&mut self, o: Optional) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        if let Some(value) = self.get_option_all(o)? {
            Ok(value)
        } else {
            self.try_to_help()?;
            self.is_empty()?;
            Err(Error::new(
                self.help.clone(),
                ErrorKind::MissingOption,
                ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                self.options.cap_mode,
            ))
        }
    }

    fn require_option_until<'a, T: FromStr>(&mut self, o: Optional, limit: usize) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.state.proceed(MemoryState::ProcessingOptionals);
        let values = self.require_option_all(o)?;
        // verify the size of the vector does not exceed `n`
        match values.len() <= limit {
            true => Ok(values),
            false => Err(Error::new(
                self.help.clone(),
                ErrorKind::ExceedingMaxCount,
                ErrorContext::ExceededThreshold(
                    self.known_args.pop().unwrap(),
                    values.len(),
                    limit,
                ),
                self.options.cap_mode,
            )),
        }
    }

    /// Queries if a flag was raised once and only once.
    ///
    /// Errors if the flag has an attached value or was raised multiple times.
    fn check_flag<'a>(&mut self, f: Flag) -> Result<bool> {
        self.state.proceed(MemoryState::ProcessingFlags);
        let occurences = self.check_flag_all(f)?;
        match occurences > 1 {
            true => {
                self.try_to_help()?;
                Err(Error::new(
                    self.help.clone(),
                    ErrorKind::DuplicateOptions,
                    ErrorContext::FailedArg(self.known_args.pop().unwrap()),
                    self.options.cap_mode,
                ))
            }
            // the flag was either raised once or not at all
            false => Ok(occurences == 1),
        }
    }

    /// Queries for the number of times a flag was raised.
    ///
    /// Errors if the flag has an attached value. Returning a zero indicates the flag was never raised.
    fn check_flag_all<'a>(&mut self, f: Flag) -> Result<usize> {
        self.state.proceed(MemoryState::ProcessingFlags);
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(f.get_name());
        // try to find the switch locations
        if let Some(c) = f.get_switch() {
            locs.extend(self.take_switch_locs(c));
        };
        self.known_args.push(ArgType::Flag(f));
        let mut occurences = self.pull_flag(locs, false);
        // verify there are no values attached to this flag
        if let Some(val) = occurences.iter_mut().find(|p| p.is_some()) {
            self.try_to_help()?;
            return Err(Error::new(
                self.help.clone(),
                ErrorKind::UnexpectedValue,
                ErrorContext::UnexpectedValue(self.known_args.pop().unwrap(), val.take().unwrap()),
                self.options.cap_mode,
            ));
        } else {
            let raised = occurences.len() != 0;
            // check if the user is asking for help by raising the help flag
            if let Some(hp) = &self.help {
                if raised == true
                    && ArgType::from(hp.get_arg()).into_flag().unwrap().get_name()
                        == self
                            .known_args
                            .last()
                            .unwrap()
                            .as_flag()
                            .unwrap()
                            .get_name()
                {
                    self.asking_for_help = true;
                }
            }
            // return the number of times the flag was raised
            Ok(occurences.len())
        }
    }

    /// Queries for the number of times a flag was raised up until `n` times.
    ///
    /// Errors if the flag has an attached value. Returning a zero indicates the flag was never raised.
    fn check_flag_until<'a>(&mut self, f: Flag, limit: usize) -> Result<usize> {
        self.state.proceed(MemoryState::ProcessingFlags);
        let occurences = self.check_flag_all(f)?;
        // verify the size of the vector does not exceed `n`
        match occurences <= limit {
            true => Ok(occurences),
            false => Err(Error::new(
                self.help.clone(),
                ErrorKind::ExceedingMaxCount,
                ErrorContext::ExceededThreshold(self.known_args.pop().unwrap(), occurences, limit),
                self.options.cap_mode,
            )),
        }
    }
}

// Public API

impl Cli<Memory> {
    /// Sets the [Help] information for the command-line processor.
    ///
    /// Once the help information is updated, this function returns true if help
    /// is detected on the command-line only if help is configured as a priority.
    pub fn help(&mut self, help: Help) -> Result<bool> {
        self.help = Some(help);
        // check for flag if not already raised
        if self.asking_for_help == false && self.is_help_enabled() == true {
            self.asking_for_help = self.check(self.help.as_ref().unwrap().get_arg())?;
        }
        Ok(self.asking_for_help)
    }

    /// Attempts to display the currently available help information if help was
    /// detected on the command-line.
    pub fn raise_help(&self) -> Result<()> {
        self.try_to_help()
    }

    /// Clears the status flag indicating if help was detected on the command-line
    pub fn lower_help(&mut self) -> () {
        self.asking_for_help = false;
    }

    /// Removes the current help information stored for the command-line processor.
    pub fn unset_help(&mut self) -> () {
        self.help = None;
    }

    /// Determines if an `UnattachedArg` exists to be served as a subcommand.
    ///
    /// If so, it will call `interpret` on the type defined. If not, it will return none.
    pub fn nest<'a, T: Subcommand<U>, U>(
        &mut self,
        subcommand: Arg<Callable>,
    ) -> Result<Option<T>> {
        self.known_args.push(ArgType::from(subcommand));
        // check but do not remove if an unattached arg exists
        let command_exists = self
            .tokens
            .iter()
            .find(|f| match f {
                Some(Token::UnattachedArgument(_, _)) => true,
                _ => false,
            })
            .is_some();
        if command_exists == true {
            // reset the parser state upon entering new subcommand
            self.state = MemoryState::reset();
            let sub = Some(T::interpret(self)?);
            self.state.proceed(MemoryState::ProcessingSubcommands);
            Ok(sub)
        } else {
            self.state.proceed(MemoryState::ProcessingSubcommands);
            return Ok(None);
        }
    }

    /// Tries to match the next positional argument against an array of strings in `bank`.
    ///
    /// If fails, it will attempt to offer a spelling suggestion if the name is close depending
    /// on the configured cost threshold for string alignment.
    ///
    /// Panics if there is not a next positional argument. This command should only be
    /// called immediately in the nested subcommand's `interpret(...)` method, which is
    /// triggered on a successful call to the previous command's call to `nest(...)`.
    pub fn select<T: AsRef<str> + std::cmp::PartialEq>(&mut self, bank: &[T]) -> Result<String> {
        // find the unattached arg's index before it is removed from the token stream
        let i: usize = self
            .tokens
            .iter()
            .find_map(|f| match f {
                Some(Token::UnattachedArgument(i, _)) => Some(*i),
                _ => None,
            })
            .expect("an unattached argument must exist before calling `match(...)`");
        let command = self
            .next_uarg()
            .expect("`nest(...)` must be called before this function");

        // perform partial clean to ensure no arguments are remaining behind the command (uncaught options)
        let ooc_arg = self.capture_bad_flag(i)?;

        if bank.iter().find(|p| p.as_ref() == command).is_some() {
            if let Some((prefix, key, pos)) = ooc_arg {
                if pos < i {
                    self.try_to_help()?;
                    return Err(Error::new(
                        self.help.clone(),
                        ErrorKind::OutOfContextArgSuggest,
                        ErrorContext::OutofContextArgSuggest(format!("{}{}", prefix, key), command),
                        self.options.cap_mode,
                    ));
                }
            }
            Ok(command)
        // try to offer a spelling suggestion otherwise say we've hit an unexpected argument
        } else {
            // bypass sequence alignment algorithm if threshold == 0
            if let Some(w) = if self.options.threshold > 0 {
                seqalin::sel_min_edit_str(&command, &bank, self.options.threshold)
            } else {
                None
            } {
                Err(Error::new(
                    self.help.clone(),
                    ErrorKind::SuggestSubcommand,
                    ErrorContext::SuggestWord(command, w.to_string()),
                    self.options.cap_mode,
                ))
            } else {
                self.try_to_help()?;
                Err(Error::new(
                    self.help.clone(),
                    ErrorKind::UnknownSubcommand,
                    ErrorContext::UnknownSubcommand(
                        self.known_args.pop().expect("requires positional argument"),
                        command,
                    ),
                    self.options.cap_mode,
                ))
            }
        }
    }

    /// Returns the existence of `arg`.
    /// 
    /// - If `arg` is a flag, then it checks for the associated name.
    /// 
    /// If `arg` is found, then the result is `true`. If `arg` is not found, then 
    /// the result is `false`. 
    /// 
    /// This function errors if a value is associated with the `arg` or if the `arg`
    /// is found multiple times.
    pub fn check<'a>(&mut self, arg: Arg<Raisable>) -> Result<bool> {
        match ArgType::from(arg) {
            ArgType::Flag(fla) => self.check_flag(fla),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns the number of instances that `arg` exists.
    /// 
    /// - If `arg` is a flag, then it checks for all references of its associated name.
    /// 
    /// If `arg` is found, then the result is the number of times it is found.
    /// If `arg` is not found, then the result is 0.
    /// 
    /// This function errors if a value is associated with an instances of `arg`.
    pub fn check_all<'a>(&mut self, arg: Arg<Raisable>) -> Result<usize> {
        match ArgType::from(arg) {
            ArgType::Flag(fla) => self.check_flag_all(fla),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns the number of instances that `arg` exists, up until an amount equal to `limit`.
    /// 
    /// - If `arg` is a flag, then it checks for all references of its associated name.
    /// 
    /// If `arg` is found, then the result is the number of times it is found.
    /// If `arg` is not found, then the result is 0. The result is guaranteed to 
    /// be between 0 and no more than `limit`.
    /// 
    /// This function errors if a value is associated with an instances of `arg`.
    pub fn check_until<'a>(&mut self, arg: Arg<Raisable>, limit: usize) -> Result<usize> {
        match ArgType::from(arg) {
            ArgType::Flag(fla) => self.check_flag_until(fla, limit),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns a single value associated with `arg`, if one exists.
    /// 
    /// - If `arg` is a positional argument, then it takes the next unnamed argument.
    /// - If `arg` is an option argument, then it takes the value associated with its name.
    /// 
    /// If no value exists for `arg`, the result is `None`.
    /// 
    /// This function errors if parsing into type `T` fails or if the number of values found
    /// is greater than 1.
    pub fn get<'a, T: FromStr>(&mut self, arg: Arg<Valuable>) -> Result<Option<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.get_option(opt),
            ArgType::Positional(pos) => self.get_positional(pos),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns all values associated with `arg`, if they exist.
    /// 
    /// - If `arg` is a positional argument, then it takes all the following unnamed arguments.
    /// - If `arg` is an option argument, then it takes all the values associated with its name.
    /// 
    /// If no values exists for `arg`, the result is `None`. If values do exist,
    /// then the resulting vector is guaranteed to have `1 <= len()`.
    /// 
    /// This function errors if parsing into type `T` fails.
    pub fn get_all<'a, T: FromStr>(&mut self, arg: Arg<Valuable>) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.get_option_all(opt),
            ArgType::Positional(pos) => self.get_positional_all(pos),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns all values associated with `arg` up until an amount equal to `limit`, if they exist.
    /// 
    /// - If `arg` is a positional argument, then it takes all remaining unnamed arguments up until `limit`.  
    /// - If `arg` is an option argument, then it takes an arbitrary amount of values associated with its name up until `limit`.
    /// 
    /// If no values exists for `arg`, the result is `None`. If values do exist,
    /// then the resulting vector is guaranteed to have `1 <= len() <= limit`.
    /// 
    /// This function errors if parsing into type `T` fails or if the number of 
    /// values found exceeds the specified `limit`.
    pub fn get_until<'a, T: FromStr>(
        &mut self,
        arg: Arg<Valuable>,
        limit: usize,
    ) -> Result<Option<Vec<T>>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.get_option_until(opt, limit),
            ArgType::Positional(pos) => self.get_positional_until(pos, limit),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns a single value associated with `arg`.
    /// 
    /// - If `arg` is a positional argument, then it takes the next unnamed argument.
    /// - If `arg` is an option argument, then it takes the value associated with its name.
    /// 
    /// This function errors if parsing into type `T` fails or if the number of values found
    /// is not exactly equal to 1.
    pub fn require<'a, T: FromStr>(&mut self, arg: Arg<Valuable>) -> Result<T>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.require_option(opt),
            ArgType::Positional(pos) => self.require_positional(pos),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns all values associated with `arg`.
    /// 
    /// - If `arg` is a positional argument, then it takes all remaining unnamed arguments.  
    /// - If `arg` is an option argument, then it takes an arbitrary amount of values associated with its name.
    /// 
    /// This function errors if parsing into type `T` fails or if zero values are found.
    ///
    /// The resulting vector is guaranteed to have `1 <= len()`.
    pub fn require_all<'a, T: FromStr>(&mut self, arg: Arg<Valuable>) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.require_option_all(opt),
            ArgType::Positional(pos) => self.require_positional_all(pos),
            _ => panic!("impossible code condition"),
        }
    }

    /// Returns all values associated with `arg` up until an amount equal to `limit`.
    /// 
    /// - If `arg` is a positional argument, then it takes all remaining unnamed arguments up until `limit`.  
    /// - If `arg` is an option argument, then it takes an arbitrary amount of values associated with its name up until `limit`.
    /// 
    /// This function errors if parsing into type `T` fails, if zero values are found, or
    /// if the number of values found exceeds the specified `limit`.
    ///
    /// The resulting vector is guaranteed to have `1 <= len() <= limit`.
    pub fn require_until<'a, T: FromStr>(
        &mut self,
        arg: Arg<Valuable>,
        limit: usize,
    ) -> Result<Vec<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match ArgType::from(arg) {
            ArgType::Optional(opt) => self.require_option_until(opt, limit),
            ArgType::Positional(pos) => self.require_positional_until(pos, limit),
            _ => panic!("impossible code condition"),
        }
    }

    /// Checks that there are no more unprocessed arguments that were stored in
    /// memory.
    /// 
    /// This function errors if there are any unhandled arguments that were never
    /// requested during the [Memory] stage.
    pub fn is_empty<'a>(&'a mut self) -> Result<()> {
        self.state.proceed(MemoryState::End);
        self.try_to_help()?;
        // check if map is empty, and return the minimum found index.
        if let Some((prefix, key, _)) = self.capture_bad_flag(self.tokens.len())? {
            Err(Error::new(
                self.help.clone(),
                ErrorKind::UnexpectedArg,
                ErrorContext::UnexpectedArg(format!("{}{}", prefix, key)),
                self.options.cap_mode,
            ))
        // find first non-none token
        } else if let Some(t) = self.tokens.iter().find(|p| p.is_some()) {
            match t {
                Some(Token::UnattachedArgument(_, word)) => Err(Error::new(
                    self.help.clone(),
                    ErrorKind::UnexpectedArg,
                    ErrorContext::UnexpectedArg(word.to_string()),
                    self.options.cap_mode,
                )),
                Some(Token::Terminator(_)) => Err(Error::new(
                    self.help.clone(),
                    ErrorKind::UnexpectedArg,
                    ErrorContext::UnexpectedArg(symbol::FLAG.to_string()),
                    self.options.cap_mode,
                )),
                _ => panic!("no other tokens types should be left"),
            }
        } else {
            Ok(())
        }
    }

    /// Collects the list of arguments that were ignored due to being placed after
    /// a terminator flag (`--`).
    /// 
    /// If there are no arguments that were ignored, the result is an empty list.
    /// 
    /// This function errors if a value is found to be associated with the terminator
    /// flag.
    pub fn remainder(&mut self) -> Result<Vec<String>> {
        self.tokens
            .iter_mut()
            .skip_while(|tkn| match tkn {
                Some(Token::Terminator(_)) => false,
                _ => true,
            })
            .filter_map(|tkn| {
                match tkn {
                    // remove the terminator from the stream
                    Some(Token::Terminator(_)) => {
                        tkn.take().unwrap();
                        None
                    }
                    Some(Token::Ignore(_, _)) => Some(Ok(tkn.take().unwrap().take_str())),
                    Some(Token::AttachedArgument(_, _)) => Some(Err(Error::new(
                        self.help.clone(),
                        ErrorKind::UnexpectedValue,
                        ErrorContext::UnexpectedValue(
                            ArgType::Flag(Flag::new("")),
                            tkn.take().unwrap().take_str(),
                        ),
                        self.options.cap_mode,
                    ))),
                    _ => panic!("no other tokens should exist beyond terminator {:?}", tkn),
                }
            })
            .collect()
    }
}

// Internal methods
impl Cli<Memory> {
    /// Attempts to extract the next unattached argument to get a positional with valid parsing.
    ///
    /// Assumes the [Positional] argument is already added as the last element to the `known_args` vector.
    fn try_positional<'a, T: FromStr>(&mut self) -> Result<Option<T>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match self.next_uarg() {
            Some(word) => match word.parse::<T>() {
                Ok(r) => Ok(Some(r)),
                Err(err) => {
                    self.try_to_help()?;
                    self.prioritize_suggestion()?;
                    Err(Error::new(
                        self.help.clone(),
                        ErrorKind::BadType,
                        ErrorContext::FailedCast(
                            self.known_args.pop().unwrap(),
                            word,
                            Box::new(err),
                        ),
                        self.options.cap_mode,
                    ))
                }
            },
            None => Ok(None),
        }
    }

    /// Transforms the list of `known_args` into a list of the names for every available
    /// flag.
    ///
    /// This method is useful for acquiring a word bank to offer a flag spelling suggestion.
    fn known_args_as_flag_names(&self) -> HashSet<&str> {
        // note: collect into a `std::collections::HashSet` to avoid dupe
        self.known_args
            .iter()
            .filter_map(|f| match f {
                ArgType::Flag(f) => Some(f.get_name()),
                ArgType::Optional(o) => Some(o.get_flag().get_name()),
                _ => None,
            })
            .collect()
    }

    /// Returns the first index where a flag/switch still remains in the token stream.
    ///
    /// The flag must occur in the token stream before the `breakpoint` index. If
    /// the `opt_store` hashmap is empty, it will return none.
    fn find_first_flag_left(&self, breakpoint: usize) -> Option<(&str, usize)> {
        let mut min_i: Option<(&str, usize)> = None;
        let mut opt_it = self
            .store
            .iter()
            .filter(|(_, slot)| slot.is_visited() == false);
        while let Some((key, val)) = opt_it.next() {
            // check if this flag's index comes before the currently known minimum index
            min_i = if *val.first().unwrap() < breakpoint
                && (min_i.is_none() || min_i.unwrap().1 > *val.first().unwrap())
            {
                Some((key.as_ref(), *val.first().unwrap()))
            } else {
                min_i
            };
        }
        min_i
    }

    /// Verifies there are no uncaught flags behind a given index.
    fn capture_bad_flag<'a>(&self, i: usize) -> Result<Option<(&str, &str, usize)>> {
        if let Some((key, val)) = self.find_first_flag_left(i) {
            self.try_to_help()?;
            // check what type of token it was to determine if it was called with '-' or '--'
            if let Some(t) = self.tokens.get(val).unwrap() {
                let prefix = match t {
                    Token::Switch(_, _) | Token::EmptySwitch(_) => symbol::SWITCH,
                    Token::Flag(_) => {
                        // try to match it with a valid flag from word bank
                        let bank: Vec<&str> = self.known_args_as_flag_names().into_iter().collect();
                        if let Some(closest) = if self.options.threshold > 0 {
                            seqalin::sel_min_edit_str(key, &bank, self.options.threshold)
                        } else {
                            None
                        } {
                            return Err(Error::new(
                                self.help.clone(),
                                ErrorKind::SuggestArg,
                                ErrorContext::SuggestWord(
                                    format!("{}{}", symbol::FLAG, key),
                                    format!("{}{}", symbol::FLAG, closest),
                                ),
                                self.options.cap_mode,
                            ));
                        }
                        symbol::FLAG
                    }
                    _ => panic!("no other tokens are allowed in hashmap"),
                };
                Ok(Some((prefix, key, val)))
            } else {
                panic!("this token's values have been removed")
            }
        } else {
            Ok(None)
        }
    }

    /// Returns all locations in the token stream where the flag identifier `tag` is found.
    ///
    /// Information about Option<Vec<T>> vs. empty Vec<T>: https://users.rust-lang.org/t/space-time-usage-to-construct-vec-t-vs-option-vec-t/35596/6
    fn take_flag_locs(&mut self, tag: &str) -> Vec<usize> {
        if let Some(slot) = self.store.get_mut(&Tag::Flag(tag.to_owned())) {
            slot.visit();
            slot.get_indices().to_vec()
        } else {
            Vec::new()
        }
    }

    /// Returns all locations in the token stream where the switch identifier `c` is found.
    fn take_switch_locs(&mut self, c: &char) -> Vec<usize> {
        // allocate &str to the stack and not the heap to get from store
        let mut arr = [0; 4];
        let tag = c.encode_utf8(&mut arr);

        if let Some(slot) = self.store.get_mut(&Tag::Switch(tag.to_owned())) {
            slot.visit();
            slot.get_indices().to_vec()
        } else {
            Vec::new()
        }
    }

    /// Iterates through the list of tokens to find the first suggestion against a flag to return.
    ///
    /// Returns ok if cannot make a suggestion.
    fn prioritize_suggestion(&self) -> Result<()> {
        let mut kv: Vec<(&String, &Vec<usize>)> = self
            .store
            .iter()
            .map(|(tag, slot)| (tag.as_ref(), slot.get_indices()))
            .collect::<Vec<(&String, &Vec<usize>)>>();
        kv.sort_by(|a, b| a.1.first().unwrap().cmp(b.1.first().unwrap()));
        let bank: Vec<&str> = self.known_args_as_flag_names().into_iter().collect();
        let r = kv
            .iter()
            .find_map(|f| match self.tokens.get(*f.1.first().unwrap()).unwrap() {
                Some(Token::Flag(_)) => {
                    if let Some(word) = if self.options.threshold > 0 {
                        seqalin::sel_min_edit_str(f.0, &bank, self.options.threshold)
                    } else {
                        None
                    } {
                        Some(Error::new(
                            self.help.clone(),
                            ErrorKind::SuggestArg,
                            ErrorContext::SuggestWord(
                                format!("{}{}", symbol::FLAG, f.0),
                                format!("{}{}", symbol::FLAG, word),
                            ),
                            self.options.cap_mode,
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            });
        if self.asking_for_help == true {
            Ok(())
        } else if let Some(e) = r {
            Err(e)
        } else {
            Ok(())
        }
    }

    /// Grabs the flag/switch from the token stream, and collects.
    ///
    /// If an argument were to follow it will be in the vector.
    fn pull_flag(&mut self, locations: Vec<usize>, with_uarg: bool) -> Vec<Option<String>> {
        // remove all flag instances located at each index `i` in the vector `locations`
        locations
            .iter()
            .map(|i| {
                // remove the flag instance from the token stream
                self.tokens.get_mut(*i).unwrap().take();
                // check the next position for a value
                if let Some(t_next) = self.tokens.get_mut(*i + 1) {
                    match t_next {
                        Some(Token::AttachedArgument(_, _)) => {
                            Some(t_next.take().unwrap().take_str())
                        }
                        Some(Token::UnattachedArgument(_, _)) => {
                            // do not take unattached arguments unless told by parameter
                            match with_uarg {
                                true => Some(t_next.take().unwrap().take_str()),
                                false => None,
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Pulls the next `UnattachedArg` token from the token stream.
    ///
    /// If no more `UnattachedArg` tokens are left, it will return none.
    fn next_uarg(&mut self) -> Option<String> {
        if let Some(p) = self.tokens.iter_mut().find(|s| match s {
            Some(Token::UnattachedArgument(_, _)) | Some(Token::Terminator(_)) => true,
            _ => false,
        }) {
            if let Some(Token::Terminator(_)) = p {
                None
            } else {
                Some(p.take().unwrap().take_str())
            }
        } else {
            None
        }
    }

    /// Checks if help is enabled and is some value.
    fn is_help_enabled(&self) -> bool {
        // change to does_help_exist()
        self.help.is_some()
    }

    /// Checks if help has been raised and will return its own error for displaying
    /// help.
    fn try_to_help(&self) -> Result<()> {
        if self.options.prioritize_help == true
            && self.asking_for_help == true
            && self.is_help_enabled() == true
        {
            Err(Error::new(
                self.help.clone(),
                ErrorKind::Help,
                ErrorContext::Help,
                self.options.cap_mode,
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::ErrorKind;

    /// Helper test fn to write vec of &str as iterator for Cli parameter.
    fn args<'a>(args: Vec<&'a str>) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(args.into_iter().map(|f| f.to_string()).into_iter())
    }

    #[test]
    fn get_all_optionals() {
        // option provided multiple times
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "plan",
                "--fileset",
                "a",
                "--fileset=b",
                "--fileset",
                "c",
            ]))
            .save();
        let sets: Vec<String> = cli
            .get_option_all(Optional::new("fileset"))
            .unwrap()
            .unwrap();
        assert_eq!(sets, vec!["a", "b", "c"]);
        // failing case- bad conversion of 'c' to an integer
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "plan",
                "--digit",
                "10",
                "--digit=9",
                "--digit",
                "c",
            ]))
            .save();
        assert_eq!(
            cli.get_option_all::<i32>(Optional::new("digit")).is_err(),
            true
        ); // bad conversion
           // option provided as valid integers
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "plan",
                "--digit",
                "10",
                "--digit=9",
                "--digit",
                "1",
            ]))
            .save();
        let sets: Vec<i32> = cli.get_option_all(Optional::new("digit")).unwrap().unwrap();
        assert_eq!(sets, vec![10, 9, 1]);
        // option provided once
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "plan", "--fileset", "a"]))
            .save();
        let sets: Vec<String> = cli
            .get_option_all(Optional::new("fileset"))
            .unwrap()
            .unwrap();
        assert_eq!(sets, vec!["a"]);
        // option not provided
        let mut cli = Cli::new().parse(args(vec!["orbit", "plan"])).save();
        let sets: Option<Vec<String>> = cli.get_option_all(Optional::new("fileset")).unwrap();
        assert_eq!(sets, None);
    }

    #[test]
    fn match_command() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "get",
                "rary.gates",
                "--instance",
                "--component",
            ]))
            .save();
        // successfully matched 'get' command
        assert_eq!(
            cli.select(&["new", "get", "install", "edit"]).unwrap(),
            "get".to_string()
        );

        let mut cli = Cli::new()
            .threshold(4)
            .parse(args(vec![
                "orbit",
                "got",
                "rary.gates",
                "--instance",
                "--component",
            ]))
            .save();
        // suggest 'get' command
        assert!(cli.select(&["new", "get", "install", "edit"]).is_err());
    }

    #[test]
    #[should_panic = "requires positional argument"]
    fn match_command_no_arg() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "got",
                "rary.gates",
                "--instance",
                "--component",
            ]))
            .save();
        // cli has no positional arguments loaded to report as error... panic
        assert!(cli.select(&["new", "get", "install", "edit"]).is_err());
    }

    #[test]
    fn find_first_flag_left() {
        let cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--help",
                "new",
                "rary.gates",
                "--vcs",
                "git",
            ]))
            .save();
        assert_eq!(
            cli.find_first_flag_left(cli.tokens.len()),
            Some(("help", 0))
        );

        let cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates"]))
            .save();
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), None);

        let cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "new",
                "rary.gates",
                "--vcs",
                "git",
                "--help",
            ]))
            .save();
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), Some(("vcs", 2)));

        let cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates", "-c=git", "--help"]))
            .save();
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), Some(("c", 2)));

        let cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates", "-c=git", "--help"]))
            .save();
        assert_eq!(cli.find_first_flag_left(1), None); // check before 'rary.gates' position

        let cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--unknown",
                "new",
                "rary.gates",
                "-c=git",
                "--help",
            ]))
            .save();
        assert_eq!(cli.find_first_flag_left(1), Some(("unknown", 0))); // check before 'new' subcommand
    }

    #[test]
    fn processed_all_args() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--upgrade",
                "new",
                "rary.gates",
                "--vcs",
                "git",
            ]))
            .save();
        // tokens are still in token stream
        let _ = cli.check_flag(Flag::new("upgrade")).unwrap();
        let _: Option<String> = cli.get_option(Optional::new("vcs")).unwrap();
        let _: String = cli.require_positional(Positional::new("command")).unwrap();
        let _: String = cli.require_positional(Positional::new("ip")).unwrap();
        // no more tokens left in stream
        assert_eq!(cli.is_empty().unwrap(), ());

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates", symbol::FLAG]))
            .save();
        // removes only valid args/flags/opts
        let _ = cli.check_flag(Flag::new("help")).unwrap();
        let _: Option<String> = cli.get_option(Optional::new("vcs")).unwrap();
        let _: String = cli.require_positional(Positional::new("command")).unwrap();
        let _: String = cli.require_positional(Positional::new("ip")).unwrap();
        // unexpected '--'
        assert!(cli.is_empty().is_err());

        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--help",
                "new",
                "rary.gates",
                "--vcs",
                "git",
            ]))
            .save();
        // no tokens were removed (help will also raise error)
        assert!(cli.is_empty().is_err());

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", symbol::FLAG, "some", "extra", "words"]))
            .save();
        let _: Vec<String> = cli.remainder().unwrap();
        // terminator removed as well as its arguments that were ignored
        assert_eq!(cli.is_empty().unwrap(), ());
    }

    #[test]
    fn tokenizer() {
        let cli = Cli::new().parse(args(vec![])).save();
        assert_eq!(cli.tokens, vec![]);

        let cli = Cli::new().parse(args(vec!["orbit"])).save();
        assert_eq!(cli.tokens, vec![]);

        let cli = Cli::new().parse(args(vec!["orbit", "--help"])).save();
        assert_eq!(cli.tokens, vec![Some(Token::Flag(0))]);

        let cli = Cli::new().parse(args(vec!["orbit", "--help", "-v"])).save();
        assert_eq!(
            cli.tokens,
            vec![Some(Token::Flag(0)), Some(Token::Switch(1, 'v'))],
        );

        let cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates"]))
            .save();
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::UnattachedArgument(0, "new".to_string())),
                Some(Token::UnattachedArgument(1, "rary.gates".to_string())),
            ],
        );

        let cli = Cli::new()
            .parse(args(vec!["orbit", "--help", "-vh"]))
            .save();
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Flag(0)),
                Some(Token::Switch(1, 'v')),
                Some(Token::Switch(1, 'h')),
            ],
        );

        let cli = Cli::new()
            .parse(args(vec!["orbit", "--help", "-vhc=10"]))
            .save();
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Flag(0)),
                Some(Token::Switch(1, 'v')),
                Some(Token::Switch(1, 'h')),
                Some(Token::Switch(1, 'c')),
                Some(Token::AttachedArgument(1, "10".to_string())),
            ],
        );

        // an attached argument can sneak in behind a terminator
        let cli = Cli::new()
            .parse(args(vec!["orbit", "--=value", "extra"]))
            .save();
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Terminator(0)),
                Some(Token::AttachedArgument(0, "value".to_string())),
                Some(Token::Ignore(1, "extra".to_string())),
            ]
        );

        // final boss
        let cli = Cli::new().parse(args(vec![
            "orbit",
            "--help",
            "-v",
            "new",
            "ip",
            "--lib",
            "--name=rary.gates",
            "--help",
            "-sci",
            symbol::FLAG,
            "--map",
            "synthesis",
            "-jto",
        ]));
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Flag(0)),
                Some(Token::Switch(1, 'v')),
                Some(Token::UnattachedArgument(2, "new".to_string())),
                Some(Token::UnattachedArgument(3, "ip".to_string())),
                Some(Token::Flag(4)),
                Some(Token::Flag(5)),
                Some(Token::AttachedArgument(5, "rary.gates".to_string())),
                Some(Token::Flag(6)),
                Some(Token::Switch(7, 's')),
                Some(Token::Switch(7, 'c')),
                Some(Token::Switch(7, 'i')),
                Some(Token::Terminator(8)),
                Some(Token::Ignore(9, "--map".to_string())),
                Some(Token::Ignore(10, "synthesis".to_string())),
                Some(Token::Ignore(11, "-jto".to_string())),
            ],
        );
    }

    #[test]
    fn find_flags_and_switches() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--help",
                "-v",
                "new",
                "ip",
                "--lib",
                "--name=rary.gates",
                "--help",
                "-sci",
                "-i",
                "--",
                "--map",
                "synthesis",
                "-jto",
            ]))
            .save();

        // detects 0
        assert_eq!(cli.take_flag_locs("version"), vec![]);
        // detects 1
        assert_eq!(cli.take_flag_locs("lib"), vec![4]);
        // detects multiple
        assert_eq!(cli.take_flag_locs("help"), vec![0, 7]);
        // flag was past terminator and marked as ignore
        assert_eq!(cli.take_flag_locs("map"), vec![]);
        // filters out arguments
        assert_eq!(cli.take_flag_locs("rary.gates"), vec![]);

        // detects 0
        assert_eq!(cli.take_switch_locs(&'q'), vec![]);
        // detects 1
        assert_eq!(cli.take_switch_locs(&'v'), vec![1]);
        // detects multiple
        assert_eq!(cli.take_switch_locs(&'i'), vec![10, 11]);
        // switch was past terminator and marked as ignore
        assert_eq!(cli.take_switch_locs(&'j'), vec![]);
    }

    #[test]
    fn flags_in_map() {
        let cli = Cli::new().parse(args(vec![
            "orbit",
            "--help",
            "-v",
            "new",
            "ip",
            "--lib",
            "--name=rary.gates",
            "--help",
            "-sci",
            symbol::FLAG,
            "--map",
            "synthesis",
            "-jto",
        ]));
        let mut store = HashMap::<Tag<String>, Slot>::new();
        // store long options
        store.insert(
            Tag::Flag("help".to_string()),
            Slot {
                pointers: vec![0, 7],
                visited: false,
            },
        );
        store.insert(
            Tag::Flag("lib".to_string()),
            Slot {
                pointers: vec![4],
                visited: false,
            },
        );
        store.insert(
            Tag::Flag("name".to_string()),
            Slot {
                pointers: vec![5],
                visited: false,
            },
        );
        // stores switches too
        store.insert(
            Tag::Switch("v".to_string()),
            Slot {
                pointers: vec![1],
                visited: false,
            },
        );
        store.insert(
            Tag::Switch("s".to_string()),
            Slot {
                pointers: vec![8],
                visited: false,
            },
        );
        store.insert(
            Tag::Switch("c".to_string()),
            Slot {
                pointers: vec![9],
                visited: false,
            },
        );
        store.insert(
            Tag::Switch("i".to_string()),
            Slot {
                pointers: vec![10],
                visited: false,
            },
        );
        assert_eq!(cli.store, store);
    }

    #[test]
    fn take_unattached_args() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--help",
                "-v",
                "new",
                "ip",
                "--lib",
                "--name=rary.gates",
                "--help",
                "-scii",
                "get",
                symbol::FLAG,
                "--map",
                "synthesis",
                "-jto",
            ]))
            .save();

        assert_eq!(cli.next_uarg().unwrap(), "new".to_string());
        assert_eq!(cli.next_uarg().unwrap(), "ip".to_string());
        assert_eq!(cli.next_uarg().unwrap(), "get".to_string());
        assert_eq!(cli.next_uarg(), None);
    }

    #[test]
    fn take_remainder_args() {
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--help",
                "-v",
                "new",
                "ip",
                "--lib",
                "--name=rary.gates",
                "--help",
                "-scii",
                "get",
                symbol::FLAG,
                "--map",
                "synthesis",
                "-jto",
            ]))
            .save();
        assert_eq!(cli.remainder().unwrap(), vec!["--map", "synthesis", "-jto"]);
        // the items were removed from the token stream
        assert_eq!(cli.remainder().unwrap(), Vec::<String>::new());

        // an attached argument can sneak in behind a terminator (handle in a result fn)
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--=value", "extra"]))
            .save();
        assert!(cli.remainder().is_err());

        let mut cli = Cli::new().parse(args(vec!["orbit", "--help"])).save();
        // the terminator was never found
        assert_eq!(cli.remainder().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn pull_values_from_flags() {
        let mut cli = Cli::new().parse(args(vec!["orbit", "--help"])).save();
        let locs = cli.take_flag_locs("help");
        assert_eq!(cli.pull_flag(locs, false), vec![None]);
        assert_eq!(cli.tokens.get(0), Some(&None));

        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--name",
                "gates",
                "arg",
                "--lib",
                "new",
                "--name=gates2",
                "--opt=1",
                "--opt",
                "--help",
            ]))
            .save();
        let locs = cli.take_flag_locs("lib");
        assert_eq!(cli.pull_flag(locs, false), vec![None]);
        // token no longer exists
        assert_eq!(cli.tokens.get(3), Some(&None));

        // gets strings and removes both instances of flag from token stream
        let locs = cli.take_flag_locs("name");
        assert_eq!(
            cli.pull_flag(locs, true),
            vec![Some("gates".to_string()), Some("gates2".to_string())]
        );
        assert_eq!(cli.tokens.get(0), Some(&None));
        assert_eq!(cli.tokens.get(5), Some(&None));

        let locs = cli.take_flag_locs("opt");
        assert_eq!(cli.pull_flag(locs, true), vec![Some("1".to_string()), None]);

        // gets switches as well from the store
        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit",
                "--name",
                "gates",
                "-sicn",
                "dut",
                "new",
                "-vl=direct",
                "--help",
                "-l",
                "-m",
                "install",
            ]))
            .save();
        let locs = cli.take_switch_locs(&'l');
        assert_eq!(
            cli.pull_flag(locs, true),
            vec![Some("direct".to_string()), None]
        );
        assert_eq!(cli.tokens.get(9), Some(&None));
        assert_eq!(cli.tokens.get(12), Some(&None));
        let locs = cli.take_switch_locs(&'s');
        assert_eq!(cli.pull_flag(locs, true), vec![None]);
        let locs = cli.take_switch_locs(&'v');
        assert_eq!(cli.pull_flag(locs, true), vec![None]);
        let locs = cli.take_switch_locs(&'i');
        assert_eq!(cli.pull_flag(locs, true), vec![None]);
        let locs = cli.take_switch_locs(&'c');
        assert_eq!(cli.pull_flag(locs, false), vec![None]);
        let locs = cli.take_switch_locs(&'m');
        assert_eq!(cli.pull_flag(locs, false), vec![None]);
    }

    #[test]
    fn check_flag() {
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--help", "--verbose", "get"]))
            .save();
        assert_eq!(cli.check_flag(Flag::new("help")).unwrap(), true);
        assert_eq!(cli.check_flag(Flag::new("verbose")).unwrap(), true);
        assert_eq!(cli.check_flag(Flag::new("version")).unwrap(), false);

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--upgrade", "-u"]))
            .save();
        assert_eq!(
            cli.check_flag(Flag::new("upgrade").switch('u'))
                .unwrap_err()
                .kind(),
            ErrorKind::DuplicateOptions
        );

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--verbose", "--verbose", "--version=9"]))
            .save();
        assert_eq!(
            cli.check_flag(Flag::new("verbose")).unwrap_err().kind(),
            ErrorKind::DuplicateOptions
        );
        assert_eq!(
            cli.check_flag(Flag::new("version")).unwrap_err().kind(),
            ErrorKind::UnexpectedValue
        );
    }

    #[test]
    fn check_positional() {
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "new", "rary.gates"]))
            .save();
        assert_eq!(
            cli.get_positional::<String>(Positional::new("command"))
                .unwrap(),
            Some("new".to_string())
        );
        assert_eq!(
            cli.get_positional::<String>(Positional::new("ip")).unwrap(),
            Some("rary.gates".to_string())
        );
        assert_eq!(
            cli.get_positional::<i32>(Positional::new("path")).unwrap(),
            None
        );
    }

    #[test]
    fn check_option() {
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "command", "--rate", "10"]))
            .save();
        assert_eq!(cli.get_option(Optional::new("rate")).unwrap(), Some(10));

        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit", "--flag", "--rate=9", "command", "-r", "14",
            ]))
            .save();
        assert_eq!(
            cli.get_option::<i32>(Optional::new("rate").switch('r'))
                .unwrap_err()
                .kind(),
            ErrorKind::DuplicateOptions
        );

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--flag", "-r", "14"]))
            .save();
        assert_eq!(
            cli.get_option(Optional::new("rate").switch('r')).unwrap(),
            Some(14)
        );

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--flag", "--rate", "--verbose"]))
            .save();
        assert_eq!(
            cli.get_option::<i32>(Optional::new("rate"))
                .unwrap_err()
                .kind(),
            ErrorKind::ExpectingValue
        );

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--flag", "--rate", "five", "--verbose"]))
            .save();
        assert!(cli.get_option::<i32>(Optional::new("rate")).is_err());
    }

    #[test]
    fn take_token_str() {
        let t = Token::UnattachedArgument(0, "get".to_string());
        // consumes token and returns its internal string
        assert_eq!(t.take_str(), "get");

        let t = Token::AttachedArgument(1, "rary.gates".to_string());
        assert_eq!(t.take_str(), "rary.gates");

        let t = Token::Ignore(7, "--map".to_string());
        assert_eq!(t.take_str(), "--map");
    }

    #[test]
    #[should_panic]
    fn take_impossible_token_flag_str() {
        let t = Token::Flag(7);
        t.take_str();
    }

    #[test]
    #[should_panic]
    fn take_impossible_token_switch_str() {
        let t = Token::Switch(7, 'h');
        t.take_str();
    }

    #[test]
    #[should_panic]
    fn take_impossible_token_terminator_str() {
        let t = Token::Terminator(9);
        t.take_str();
    }

    #[test]
    fn try_help_fail() {
        let mut cli = Cli::new().parse(args(vec!["orbit", "--h"])).save();
        let locs = cli.take_flag_locs("help");
        assert_eq!(locs.len(), 0);
        assert_eq!(cli.pull_flag(locs, false), vec![]);
    }

    #[test]
    fn check_option_n() {
        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "command", "--rate", "10"]))
            .save();
        assert_eq!(
            cli.get_option_until(Optional::new("rate"), 1).unwrap(),
            Some(vec![10])
        );

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "command", "--rate", "10"]))
            .save();
        assert_eq!(
            cli.get_option_until(Optional::new("rate"), 2).unwrap(),
            Some(vec![10])
        );

        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit", "command", "--rate", "10", "--rate", "4", "--rate", "3",
            ]))
            .save();
        assert_eq!(
            cli.get_option_until::<u8>(Optional::new("rate"), 2)
                .is_err(),
            true
        );
    }

    #[test]
    fn check_flag_n() {
        let mut cli = Cli::new().parse(args(vec!["orbit"])).save();
        assert_eq!(cli.check_flag_until(Flag::new("debug"), 4).unwrap(), 0);

        let mut cli = Cli::new().parse(args(vec!["orbit", "--debug"])).save();
        assert_eq!(cli.check_flag_until(Flag::new("debug"), 1).unwrap(), 1);

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--debug", "--debug"]))
            .save();
        assert_eq!(cli.check_flag_until(Flag::new("debug"), 3).unwrap(), 2);

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--debug", "--debug", "--debug"]))
            .save();
        assert_eq!(cli.check_flag_until(Flag::new("debug"), 3).unwrap(), 3);

        let mut cli = Cli::new()
            .parse(args(vec![
                "orbit", "--debug", "--debug", "--debug", "--debug",
            ]))
            .save();
        assert_eq!(cli.check_flag_until(Flag::new("debug"), 3).is_err(), true);
    }

    #[test]
    fn check_flag_all() {
        let mut cli = Cli::new().parse(args(vec!["orbit"])).save();
        assert_eq!(cli.check_flag_all(Flag::new("debug")).unwrap(), 0);

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--debug", "--debug", "--debug"]))
            .save();
        assert_eq!(cli.check_flag_all(Flag::new("debug")).unwrap(), 3);

        let mut cli = Cli::new()
            .parse(args(vec!["orbit", "--debug", "--debug", "--debug=1"]))
            .save();
        assert_eq!(cli.check_flag_all(Flag::new("debug")).is_err(), true);
    }

    #[test]
    fn requires_positional_all() {
        let mut cli = Cli::new().parse(args(vec!["sum", "10", "20", "30"])).save();
        assert_eq!(
            cli.require_positional_all::<u8>(Positional::new("digit"))
                .unwrap(),
            vec![10, 20, 30]
        );

        let mut cli = Cli::new().parse(args(vec!["sum"])).save();
        assert_eq!(
            cli.require_positional_all::<u8>(Positional::new("digit"))
                .unwrap_err()
                .kind(),
            ErrorKind::MissingPositional
        );

        let mut cli = Cli::new()
            .parse(args(vec!["sum", "10", "--option", "20", "30"]))
            .save();
        // note: remember to always check options and flags before positional arguments
        assert_eq!(
            cli.get_option::<u8>(Optional::new("option")).unwrap(),
            Some(20)
        );
        assert_eq!(
            cli.require_positional_all::<u8>(Positional::new("digit"))
                .unwrap(),
            vec![10, 30]
        );

        let mut cli = Cli::new().parse(args(vec!["sum", "100"])).save();
        assert_eq!(
            cli.require_positional_all::<u8>(Positional::new("digit"))
                .unwrap(),
            vec![100]
        );
    }
}
