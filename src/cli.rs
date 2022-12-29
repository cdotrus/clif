use crate::arg::*;
use crate::command::FromCli;
use crate::error::{Error, ErrorContext, ErrorKind};
use crate::help::Help;
use crate::seqalin;
use crate::seqalin::Cost;
use std::collections::HashMap;
use std::str::FromStr;

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
pub struct Cli<'c> {
    tokens: Vec<Option<Token>>,
    opt_store: HashMap<Tag<String>, Vec<usize>>,
    known_args: Vec<Arg<'c>>,
    help: Option<Help<'c>>,
    asking_for_help: bool,
    prioritize_help: bool,
    threshold: Cost,
    use_color: bool,
}

impl<'c> Cli<'c> {
    /// Creates a minimal `Cli` struct.
    pub fn new() -> Self {
        Cli {
            tokens: Vec::new(),
            opt_store: HashMap::new(),
            known_args: Vec::new(),
            help: None,
            asking_for_help: false,
            prioritize_help: true,
            threshold: 0,
            use_color: true,
        }
    }

    /// Builds the `Cli` struct by perfoming lexical analysis on the vector of
    /// `String`.
    pub fn tokenize<T: Iterator<Item = String>>(mut self, args: T) -> Self {
        let mut tokens = Vec::<Option<Token>>::new();
        let mut store = HashMap::new();
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
                            .or_insert(Vec::new())
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
                            .or_insert(Vec::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::Switch(i, c)));
                    } else {
                        store
                            .entry(Tag::Switch(String::new()))
                            .or_insert(Vec::new())
                            .push(tokens.len());
                        tokens.push(Some(Token::EmptySwitch(i)));
                    }
                    // continuously split switches into individual components
                    while let Some(c) = arg.next() {
                        store
                            .entry(Tag::Switch(c.to_string()))
                            .or_insert(Vec::new())
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
        self.opt_store = store;
        self
    }

    /// Sets the maximum threshold value when comparing strings for character similiarity.
    pub fn threshold(mut self, cost: Cost) -> Self {
        self.threshold = cost;
        self
    }

    /// Enables the coloring for error messages.
    /// 
    /// This is enabled by default. Note this function is not able to override
    /// the crayon crate's color variable.
    #[cfg(feature = "color")]
    pub fn use_color(mut self) -> Self {
        self.use_color = true;
        self
    }

    /// Disables the coloring for error messages.
    /// 
    /// Note this function is only effective if the overall crayon crate's color is enabled.
    #[cfg(feature = "color")]
    pub fn disable_color(mut self) -> Self {
        self.use_color = false;
        self
    }

    /// Sets the help `text` to display when detecting `--help, -h` on the command-line.
    ///
    /// If the help text has a line describing overall usage, you can specify it with `usage_line`.
    /// This value the 0-indexed line to print when a missing positional error occurs.
    pub fn help(&mut self, help: Help<'c>) -> Result<(), Error<'c>> {
        self.help = Some(help);
        // check for flag if not already raised
        if self.asking_for_help == false && self.is_help_enabled() == true {
            self.asking_for_help =
                self.check_flag(self.help.as_ref().unwrap().get_flag().clone())?;
        }
        Ok(())
    }

    /// Checks if help is enabled and is some value.
    pub fn is_help_enabled(&self) -> bool {
        self.help.is_some()
    }

    /// Removes the current help text set for the command-line argument parser.
    pub fn disable_help(&mut self) -> () {
        self.help = None;
    }

    /// Downplays the help action to not become a priority error over other errors in the parsing.
    ///
    /// Help is prioritized by default.
    pub fn downplay_help(mut self) -> Self {
        self.prioritize_help = false;
        self
    }

    /// Prioritizes the help action over other errors during parsing.
    ///
    /// This is enabled by default.
    pub fn emphasize_help(mut self) -> Self {
        self.prioritize_help = true;
        self
    }

    /// Checks if help has been raised and will return its own error for displaying
    /// help.
    fn prioritize_help(&self) -> Result<(), Error<'c>> {
        if self.prioritize_help == true
            && self.asking_for_help == true
            && self.is_help_enabled() == true
        {
            Err(Error::new(self.help.clone(), ErrorKind::Help, ErrorContext::Help, self.use_color))
        } else {
            Ok(())
        }
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

    /// Determines if an `UnattachedArg` exists to be served as a subcommand.
    ///
    /// If so, it will call `from_cli` on the type defined. If not, it will return none.
    pub fn check_command<'a, T: FromCli>(
        &mut self,
        p: Positional<'c>,
    ) -> Result<Option<T>, Error<'_>> {
        self.known_args.push(Arg::Positional(p));
        // check but do not remove if an unattached arg exists
        let command_exists = self
            .tokens
            .iter()
            .find(|f| match f {
                Some(Token::UnattachedArgument(_, _)) => true,
                _ => false,
            })
            .is_some();
        if command_exists {
            Ok(Some(T::from_cli(self)?))
        } else {
            return Ok(None);
        }
    }

    /// Tries to match the next `UnattachedArg` with a list of given `words`.
    ///
    /// If fails, it will attempt to offer a spelling suggestion if the name is close.
    ///
    /// Panics if there is not a next `UnattachedArg`. It is recommended to not directly call
    /// this command, but through a `from_cli` call after `check_command` has been issued.
    pub fn match_command<T: AsRef<str> + std::cmp::PartialEq>(
        &mut self,
        words: &[T],
    ) -> Result<String, Error<'c>> {
        // find the unattached arg's index before it is removed from the token stream
        let i: usize = self
            .tokens
            .iter()
            .find_map(|f| match f {
                Some(Token::UnattachedArgument(i, _)) => Some(*i),
                _ => None,
            })
            .expect("an unattached argument must exist before calling `match_command`");
        let command = self
            .next_uarg()
            .expect("`check_command` must be called before this function");
        // perform partial clean to ensure no arguments are remaining behind the command (uncaught options)
        let ooc_arg = self.capture_bad_flag(i)?;

        if words.iter().find(|p| p.as_ref() == command).is_some() {
            if let Some((prefix, key, pos)) = ooc_arg {
                if pos < i {
                    self.prioritize_help()?;
                    return Err(Error::new(
                        self.help.clone(), 
                        ErrorKind::OutOfContextArgSuggest, 
                        ErrorContext::OutofContextArgSuggest(
                            format!("{}{}", prefix, key),
                            command,
                        ),
                        self.use_color,
                    ));
                }
            }
            Ok(command)
        // try to offer a spelling suggestion otherwise say we've hit an unexpected argument
        } else {
            // bypass sequence alignment algorithm if threshold == 0
            if let Some(w) = if self.threshold > 0 {
                seqalin::sel_min_edit_str(&command, &words, self.threshold)
            } else {
                None
            } {
                Err(Error::new(self.help.clone(), ErrorKind::SuggestSubcommand, ErrorContext::SuggestWord(command, w.to_string()), self.use_color))
            } else {
                self.prioritize_help()?;
                Err(Error::new(
                    self.help.clone(), 
                    ErrorKind::UnknownSubcommand, 
                    ErrorContext::UnknownSubcommand(
                        self.known_args.pop().expect("requires positional argument"),
                        command,
                    ),
                    self.use_color,
                ))
            }
        }
    }

    /// Serves the next `Positional` value in the token stream parsed as `T`.
    ///
    /// Errors if parsing fails. If the next argument is not a positional, it will
    /// not move forward in the token stream.
    pub fn check_positional<'a, T: FromStr>(
        &mut self,
        p: Positional<'c>,
    ) -> Result<Option<T>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        self.known_args.push(Arg::Positional(p));
        self.try_positional()
    }

    /// Attempts to extract the next unattached argument to get a positional with valid parsing.
    /// 
    /// Assumes the [Positional] argument is already added as the last element to the `known_args` vector.
    fn try_positional<'a, T: FromStr>(
        &mut self,
    ) -> Result<Option<T>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        match self.next_uarg() {
            Some(word) => match word.parse::<T>() {
                Ok(r) => Ok(Some(r)),
                Err(err) => {
                    self.prioritize_help()?;
                    self.prioritize_suggestion()?;
                    Err(Error::new(
                        self.help.clone(), 
                        ErrorKind::BadType, 
                        ErrorContext::FailedCast(
                            self.known_args.pop().unwrap(),
                            word,
                            Box::new(err),
                        ),
                        self.use_color,
                    ))
                }
            },
            None => Ok(None),
        }
    }

    /// Forces the next [Positional] to exist from token stream.
    ///
    /// Errors if parsing fails or if no unattached argument is left in the token stream.
    pub fn require_positional<'a, T: FromStr>(&mut self, p: Positional<'c>) -> Result<T, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        if let Some(value) = self.check_positional(p)? {
            Ok(value)
        } else {
            self.prioritize_help()?;
            self.is_empty()?;
            Err(Error::new(
                self.help.clone(), 
                ErrorKind::MissingPositional, 
                ErrorContext::FailedArg(
                    self.known_args.pop().unwrap(),
                ),
                self.use_color,
            ))
        }
    }

    /// Forces all the next [Positional] to be captured from the token stream.
    /// 
    /// Errors if parsing fails or if zero unattached arguments are left in the token stream to begin.
    /// 
    /// The resulting vector is guaranteed to have `.len() >= 1`.
    pub fn require_positional_all<'a, T: FromStr>(&mut self, p: Positional<'c>) -> Result<Vec<T>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        let mut result = Vec::<T>::new();
        result.push(self.require_positional(p)?);
        while let Some(v) = self.try_positional()? {
            result.push(v);
        }
        Ok(result)
    }

    /// Iterates through the list of tokens to find the first suggestion against a flag to return.
    ///
    /// Returns ok if cannot make a suggestion.
    fn prioritize_suggestion(&self) -> Result<(), Error<'c>> {
        let mut kv: Vec<(&String, &Vec<usize>)> = self
            .opt_store
            .iter()
            .map(|s| (s.0.as_ref(), s.1))
            .collect::<Vec<(&String, &Vec<usize>)>>();
        kv.sort_by(|a, b| a.1.first().unwrap().cmp(b.1.first().unwrap()));
        let bank = self.known_args_as_flag_names();
        let r = kv
            .iter()
            .find_map(|f| match self.tokens.get(*f.1.first().unwrap()).unwrap() {
                Some(Token::Flag(_)) => {
                    if let Some(word) = if self.threshold > 0 {
                        seqalin::sel_min_edit_str(f.0, &bank, self.threshold)
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
                            self.use_color,
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

    /// Queries for a value of `Optional`.
    ///
    /// Errors if there are multiple values or if parsing fails.
    pub fn check_option<'a, T: FromStr>(&mut self, o: Optional<'c>) -> Result<Option<T>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(o.get_flag().get_name());
        if let Some(c) = o.get_flag().get_switch() {
            locs.extend(self.take_switch_locs(c));
        }
        self.known_args.push(Arg::Optional(o));
        // pull values from where the option flags were found (including switch)
        let mut values = self.pull_flag(locs, true);
        match values.len() {
            1 => {
                if let Some(word) = values.pop().unwrap() {
                    let result = word.parse::<T>();
                    match result {
                        Ok(r) => Ok(Some(r)),
                        Err(err) => {
                            self.prioritize_help()?;
                            Err(Error::new(
                                self.help.clone(), 
                                ErrorKind::BadType, 
                                ErrorContext::FailedCast(
                                    self.known_args.pop().unwrap(),
                                    word,
                                    Box::new(err),
                                ),
                                self.use_color,
                            ))
                        }
                    }
                } else {
                    self.prioritize_help()?;
                    Err(Error::new(
                        self.help.clone(), 
                        ErrorKind::ExpectingValue, 
                        ErrorContext::FailedArg(
                            self.known_args.pop().unwrap(),
                        ),
                        self.use_color,
                    ))
                }
            }
            0 => Ok(None),
            _ => {
                self.prioritize_help()?;
                Err(Error::new(
                    self.help.clone(), 
                    ErrorKind::DuplicateOptions, 
                    ErrorContext::FailedArg(
                        self.known_args.pop().unwrap(),
                    ), 
                    self.use_color,
                ))
            }
        }
    }

    /// Queries for up to `n` values behind an `Optional`.
    ///
    /// Errors if a parsing fails from string or if the number of detected optionals is > n.
    pub fn check_option_n<'a, T: FromStr>(
        &mut self,
        o: Optional<'c>,
        n: usize,
    ) -> Result<Option<Vec<T>>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        let values = self.check_option_all::<T>(o)?;
        match values {
            // verify the size of the vector does not exceed `n`
            Some(r) => match r.len() <= n {
                true => Ok(Some(r)),
                false => Err(Error::new(
                    self.help.clone(), 
                    ErrorKind::ExceedingMaxCount, 
                    ErrorContext::ExceededThreshold(
                        self.known_args.pop().unwrap(),
                        r.len(),
                        n,
                    ),
                    self.use_color,
                )),
            },
            None => Ok(None),
        }
    }

    /// Queries for all values behind an `Optional`.
    ///
    /// Errors if a parsing fails from string.
    pub fn check_option_all<'a, T: FromStr>(
        &mut self,
        o: Optional<'c>,
    ) -> Result<Option<Vec<T>>, Error<'c>>
    where
        <T as FromStr>::Err: 'static + std::error::Error,
    {
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(o.get_flag().get_name());
        if let Some(c) = o.get_flag().get_switch() {
            locs.extend(self.take_switch_locs(c));
        }
        self.known_args.push(Arg::Optional(o));
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
                        self.prioritize_help()?;
                        return Err(Error::new(
                            self.help.clone(), 
                            ErrorKind::BadType, 
                            ErrorContext::FailedCast(
                                self.known_args.pop().unwrap(),
                                word,
                                Box::new(err),
                            ),
                            self.use_color,
                        ));
                    }
                }
            } else {
                self.prioritize_help()?;
                return Err(Error::new(
                    self.help.clone(), 
                    ErrorKind::ExpectingValue, 
                    ErrorContext::FailedArg(
                        self.known_args.pop().unwrap(),
                    ),
                    self.use_color,
                ));
            }
        }
        Ok(Some(transform))
    }

    /// Queries if a flag was raised once and only once.
    ///
    /// Errors if the flag has an attached value or was raised multiple times.
    pub fn check_flag<'a>(&mut self, f: Flag<'c>) -> Result<bool, Error<'c>> {
        let occurences = self.check_flag_all(f)?;
        match occurences > 1 {
            true => {
                self.prioritize_help()?;
                Err(Error::new(
                    self.help.clone(), 
                    ErrorKind::DuplicateOptions, 
                    ErrorContext::FailedArg(
                        self.known_args.pop().unwrap(),
                    ),
                    self.use_color,
                ))
            }
            // the flag was either raised once or not at all
            false => Ok(occurences == 1),
        }
    }

    /// Queries for the number of times a flag was raised.
    ///
    /// Errors if the flag has an attached value. Returning a zero indicates the flag was never raised.
    pub fn check_flag_all<'a>(&mut self, f: Flag<'c>) -> Result<usize, Error<'c>> {
        // collect information on where the flag can be found
        let mut locs = self.take_flag_locs(f.get_name());
        // try to find the switch locations
        if let Some(c) = f.get_switch() {
            locs.extend(self.take_switch_locs(c));
        };
        self.known_args.push(Arg::Flag(f));
        let mut occurences = self.pull_flag(locs, false);
        // verify there are no values attached to this flag
        if let Some(val) = occurences.iter_mut().find(|p| p.is_some()) {
            self.prioritize_help()?;
            return Err(Error::new(
                self.help.clone(), 
                ErrorKind::UnexpectedValue, 
                ErrorContext::UnexpectedValue(
                    self.known_args.pop().unwrap(),
                    val.take().unwrap(),
                ),
                self.use_color,
            ));
        } else {
            let raised = occurences.len() != 0;
            // check if the user is asking for help by raising the help flag
            if let Some(hp) = &self.help {
                if raised == true
                    && hp.get_flag().get_name()
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
    pub fn check_flag_n<'a>(&mut self, f: Flag<'c>, n: usize) -> Result<usize, Error<'c>> {
        let occurences = self.check_flag_all(f)?;
        // verify the size of the vector does not exceed `n`
        match occurences <= n {
            true => Ok(occurences),
            false => Err(Error::new(
                self.help.clone(), 
                ErrorKind::ExceedingMaxCount, 
                ErrorContext::ExceededThreshold(
                    self.known_args.pop().unwrap(),
                    occurences,
                    n,
                ),
                self.use_color,
            )),
        }
    }

    /// Transforms the list of `known_args` into a list of the names for every available
    /// flag.
    ///
    /// This method is useful for acquiring a word bank to offer a flag spelling suggestion.
    fn known_args_as_flag_names(&self) -> Vec<&str> {
        self.known_args
            .iter()
            .filter_map(|f| match f {
                Arg::Flag(f) => Some(f.get_name()),
                Arg::Optional(o) => Some(o.get_flag().get_name()),
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
        let mut opt_it = self.opt_store.iter();
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
    fn capture_bad_flag<'a>(&self, i: usize) -> Result<Option<(&str, &str, usize)>, Error<'c>> {
        if let Some((key, val)) = self.find_first_flag_left(i) {
            self.prioritize_help()?;
            // check what type of token it was to determine if it was called with '-' or '--'
            if let Some(t) = self.tokens.get(val).unwrap() {
                let prefix = match t {
                    Token::Switch(_, _) | Token::EmptySwitch(_) => symbol::SWITCH,
                    Token::Flag(_) => {
                        // try to match it with a valid flag from word bank
                        let bank = self.known_args_as_flag_names();
                        if let Some(closest) = if self.threshold > 0 {
                            seqalin::sel_min_edit_str(key, &bank, self.threshold)
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
                                self.use_color,
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

    /// Verifies there are no more tokens remaining in the stream.
    ///
    /// Note this mutates the referenced self only if an error is found.
    pub fn is_empty<'a>(&'a self) -> Result<(), Error<'c>> {
        self.prioritize_help()?;
        // check if map is empty, and return the minimum found index.
        if let Some((prefix, key, _)) = self.capture_bad_flag(self.tokens.len())? {
            Err(Error::new(
                self.help.clone(), 
                ErrorKind::UnexpectedArg, 
                ErrorContext::UnexpectedArg(
                    format!("{}{}", prefix, key)
                ),
                self.use_color,
            ))
        // find first non-none token
        } else if let Some(t) = self.tokens.iter().find(|p| p.is_some()) {
            match t {
                Some(Token::UnattachedArgument(_, word)) => {
                    Err(Error::new(
                        self.help.clone(), 
                        ErrorKind::UnexpectedArg, 
                        ErrorContext::UnexpectedArg(
                            word.to_string()
                        ),
                        self.use_color,
                    ))
                }
                Some(Token::Terminator(_)) => {
                    Err(Error::new(
                        self.help.clone(), 
                        ErrorKind::UnexpectedArg, 
                        ErrorContext::UnexpectedArg(
                            symbol::FLAG.to_string()
                        ),
                        self.use_color,
                    ))
                }
                _ => panic!("no other tokens types should be left"),
            }
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

    /// Removes the ignored tokens from the stream, if they exist.
    ///
    /// Errors if an `AttachedArg` is found (could only be immediately after terminator)
    /// after the terminator.
    pub fn check_remainder(&mut self) -> Result<Vec<String>, Error<'c>> {
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
                            Arg::Flag(Flag::new("")),
                            tkn.take().unwrap().take_str(),
                        ),
                        self.use_color,
                    ))),
                    _ => panic!("no other tokens should exist beyond terminator {:?}", tkn),
                }
            })
            .collect()
    }

    /// Returns all locations in the token stream where the flag identifier `tag` is found.
    ///
    /// Information about Option<Vec<T>> vs. empty Vec<T>: https://users.rust-lang.org/t/space-time-usage-to-construct-vec-t-vs-option-vec-t/35596/6
    fn take_flag_locs(&mut self, tag: &str) -> Vec<usize> {
        self.opt_store
            .remove(&Tag::Flag(tag.to_owned()))
            .unwrap_or(vec![])
    }

    /// Returns all locations in the token stream where the switch identifier `c` is found.
    fn take_switch_locs(&mut self, c: &char) -> Vec<usize> {
        // allocate &str to the stack and not the heap to get from store
        let mut arr = [0; 4];
        let tag = c.encode_utf8(&mut arr);
        self.opt_store
            .remove(&Tag::Switch(tag.to_owned()))
            .unwrap_or(vec![])
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
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "plan",
            "--fileset",
            "a",
            "--fileset=b",
            "--fileset",
            "c",
        ]));
        let sets: Vec<String> = cli
            .check_option_all(Optional::new("fileset"))
            .unwrap()
            .unwrap();
        assert_eq!(sets, vec!["a", "b", "c"]);
        // failing case- bad conversion of 'c' to an integer
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "plan",
            "--digit",
            "10",
            "--digit=9",
            "--digit",
            "c",
        ]));
        assert_eq!(
            cli.check_option_all::<i32>(Optional::new("digit")).is_err(),
            true
        ); // bad conversion
           // option provided as valid integers
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "plan",
            "--digit",
            "10",
            "--digit=9",
            "--digit",
            "1",
        ]));
        let sets: Vec<i32> = cli
            .check_option_all(Optional::new("digit"))
            .unwrap()
            .unwrap();
        assert_eq!(sets, vec![10, 9, 1]);
        // option provided once
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "plan", "--fileset", "a"]));
        let sets: Vec<String> = cli
            .check_option_all(Optional::new("fileset"))
            .unwrap()
            .unwrap();
        assert_eq!(sets, vec!["a"]);
        // option not provided
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "plan"]));
        let sets: Option<Vec<String>> = cli.check_option_all(Optional::new("fileset")).unwrap();
        assert_eq!(sets, None);
    }

    #[test]
    fn match_command() {
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "get",
            "rary.gates",
            "--instance",
            "--component",
        ]));
        // successfully matched 'get' command
        assert_eq!(
            cli.match_command(&["new", "get", "install", "edit"])
                .unwrap(),
            "get".to_string()
        );

        let mut cli = Cli::new().threshold(4).tokenize(args(vec![
            "orbit",
            "got",
            "rary.gates",
            "--instance",
            "--component",
        ]));
        // suggest 'get' command
        assert!(cli
            .match_command(&["new", "get", "install", "edit"])
            .is_err());
    }

    #[test]
    #[should_panic = "requires positional argument"]
    fn match_command_no_arg() {
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "got",
            "rary.gates",
            "--instance",
            "--component",
        ]));
        // cli has no positional arguments loaded to report as error... panic
        assert!(cli
            .match_command(&["new", "get", "install", "edit"])
            .is_err());
    }

    #[test]
    fn find_first_flag_left() {
        let cli = Cli::new().tokenize(args(vec![
            "orbit",
            "--help",
            "new",
            "rary.gates",
            "--vcs",
            "git",
        ]));
        assert_eq!(
            cli.find_first_flag_left(cli.tokens.len()),
            Some(("help", 0))
        );

        let cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates"]));
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), None);

        let cli = Cli::new().tokenize(args(vec![
            "orbit",
            "new",
            "rary.gates",
            "--vcs",
            "git",
            "--help",
        ]));
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), Some(("vcs", 2)));

        let cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates", "-c=git", "--help"]));
        assert_eq!(cli.find_first_flag_left(cli.tokens.len()), Some(("c", 2)));

        let cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates", "-c=git", "--help"]));
        assert_eq!(cli.find_first_flag_left(1), None); // check before 'rary.gates' position

        let cli = Cli::new().tokenize(args(vec![
            "orbit",
            "--unknown",
            "new",
            "rary.gates",
            "-c=git",
            "--help",
        ]));
        assert_eq!(cli.find_first_flag_left(1), Some(("unknown", 0))); // check before 'new' subcommand
    }

    #[test]
    fn processed_all_args() {
        let mut cli = Cli::new().tokenize(args(vec![
            "orbit",
            "--upgrade",
            "new",
            "rary.gates",
            "--vcs",
            "git",
        ]));
        // tokens are still in token stream
        let _ = cli.check_flag(Flag::new("upgrade")).unwrap();
        let _: Option<String> = cli.check_option(Optional::new("vcs")).unwrap();
        let _: String = cli.require_positional(Positional::new("command")).unwrap();
        let _: String = cli.require_positional(Positional::new("ip")).unwrap();
        // no more tokens left in stream
        assert_eq!(cli.is_empty().unwrap(), ());

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates", symbol::FLAG]));
        // removes only valid args/flags/opts
        let _ = cli.check_flag(Flag::new("help")).unwrap();
        let _: Option<String> = cli.check_option(Optional::new("vcs")).unwrap();
        let _: String = cli.require_positional(Positional::new("command")).unwrap();
        let _: String = cli.require_positional(Positional::new("ip")).unwrap();
        // unexpected '--'
        assert!(cli.is_empty().is_err());

        let cli = Cli::new().tokenize(args(vec![
            "orbit",
            "--help",
            "new",
            "rary.gates",
            "--vcs",
            "git",
        ]));
        // no tokens were removed (help will also raise error)
        assert!(cli.is_empty().is_err());

        let mut cli =
            Cli::new().tokenize(args(vec!["orbit", symbol::FLAG, "some", "extra", "words"]));
        let _: Vec<String> = cli.check_remainder().unwrap();
        // terminator removed as well as its arguments that were ignored
        assert_eq!(cli.is_empty().unwrap(), ());
    }

    #[test]
    fn tokenizer() {
        let cli = Cli::new().tokenize(args(vec![]));
        assert_eq!(cli.tokens, vec![]);

        let cli = Cli::new().tokenize(args(vec!["orbit"]));
        assert_eq!(cli.tokens, vec![]);

        let cli = Cli::new().tokenize(args(vec!["orbit", "--help"]));
        assert_eq!(cli.tokens, vec![Some(Token::Flag(0))]);

        let cli = Cli::new().tokenize(args(vec!["orbit", "--help", "-v"]));
        assert_eq!(
            cli.tokens,
            vec![Some(Token::Flag(0)), Some(Token::Switch(1, 'v'))],
        );

        let cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates"]));
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::UnattachedArgument(0, "new".to_string())),
                Some(Token::UnattachedArgument(1, "rary.gates".to_string())),
            ],
        );

        let cli = Cli::new().tokenize(args(vec!["orbit", "--help", "-vh"]));
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Flag(0)),
                Some(Token::Switch(1, 'v')),
                Some(Token::Switch(1, 'h')),
            ],
        );

        let cli = Cli::new().tokenize(args(vec!["orbit", "--help", "-vhc=10"]));
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
        let cli = Cli::new().tokenize(args(vec!["orbit", "--=value", "extra"]));
        assert_eq!(
            cli.tokens,
            vec![
                Some(Token::Terminator(0)),
                Some(Token::AttachedArgument(0, "value".to_string())),
                Some(Token::Ignore(1, "extra".to_string())),
            ]
        );

        // final boss
        let cli = Cli::new().tokenize(args(vec![
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
        let mut cli = Cli::new().tokenize(args(vec![
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
        ]));

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
        let cli = Cli::new().tokenize(args(vec![
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
        let mut opt_store = HashMap::<Tag<String>, Vec<usize>>::new();
        // store long options
        opt_store.insert(Tag::Flag("help".to_string()), vec![0, 7]);
        opt_store.insert(Tag::Flag("lib".to_string()), vec![4]);
        opt_store.insert(Tag::Flag("name".to_string()), vec![5]);
        // stores switches too
        opt_store.insert(Tag::Switch("v".to_string()), vec![1]);
        opt_store.insert(Tag::Switch("s".to_string()), vec![8]);
        opt_store.insert(Tag::Switch("c".to_string()), vec![9]);
        opt_store.insert(Tag::Switch("i".to_string()), vec![10]);
        assert_eq!(cli.opt_store, opt_store);
    }

    #[test]
    fn take_unattached_args() {
        let mut cli = Cli::new().tokenize(args(vec![
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
        ]));

        assert_eq!(cli.next_uarg().unwrap(), "new".to_string());
        assert_eq!(cli.next_uarg().unwrap(), "ip".to_string());
        assert_eq!(cli.next_uarg().unwrap(), "get".to_string());
        assert_eq!(cli.next_uarg(), None);
    }

    #[test]
    fn take_remainder_args() {
        let mut cli = Cli::new().tokenize(args(vec![
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
        ]));
        assert_eq!(
            cli.check_remainder().unwrap(),
            vec!["--map", "synthesis", "-jto"]
        );
        // the items were removed from the token stream
        assert_eq!(cli.check_remainder().unwrap(), Vec::<String>::new());

        // an attached argument can sneak in behind a terminator (handle in a result fn)
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--=value", "extra"]));
        assert!(cli.check_remainder().is_err());

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--help"]));
        // the terminator was never found
        assert_eq!(cli.check_remainder().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn pull_values_from_flags() {
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--help"]));
        let locs = cli.take_flag_locs("help");
        assert_eq!(cli.pull_flag(locs, false), vec![None]);
        assert_eq!(cli.tokens.get(0), Some(&None));

        let mut cli = Cli::new().tokenize(args(vec![
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
        ]));
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
        let mut cli = Cli::new().tokenize(args(vec![
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
        ]));
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
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--help", "--verbose", "get"]));
        assert_eq!(cli.check_flag(Flag::new("help")).unwrap(), true);
        assert_eq!(cli.check_flag(Flag::new("verbose")).unwrap(), true);
        assert_eq!(cli.check_flag(Flag::new("version")).unwrap(), false);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--upgrade", "-u"]));
        assert_eq!(
            cli.check_flag(Flag::new("upgrade").switch('u'))
                .unwrap_err()
                .kind(),
            ErrorKind::DuplicateOptions
        );

        let mut cli =
            Cli::new().tokenize(args(vec!["orbit", "--verbose", "--verbose", "--version=9"]));
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
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "new", "rary.gates"]));
        assert_eq!(
            cli.check_positional::<String>(Positional::new("command"))
                .unwrap(),
            Some("new".to_string())
        );
        assert_eq!(
            cli.check_positional::<String>(Positional::new("ip"))
                .unwrap(),
            Some("rary.gates".to_string())
        );
        assert_eq!(
            cli.check_positional::<i32>(Positional::new("path"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn check_option() {
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "command", "--rate", "10"]));
        assert_eq!(cli.check_option(Optional::new("rate")).unwrap(), Some(10));

        let mut cli = Cli::new().tokenize(args(vec![
            "orbit", "--flag", "--rate=9", "command", "-r", "14",
        ]));
        assert_eq!(
            cli.check_option::<i32>(Optional::new("rate").switch('r'))
                .unwrap_err()
                .kind(),
            ErrorKind::DuplicateOptions
        );

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--flag", "-r", "14"]));
        assert_eq!(
            cli.check_option(Optional::new("rate").switch('r')).unwrap(),
            Some(14)
        );

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--flag", "--rate", "--verbose"]));
        assert_eq!(
            cli.check_option::<i32>(Optional::new("rate"))
                .unwrap_err()
                .kind(),
            ErrorKind::ExpectingValue
        );

        let mut cli =
            Cli::new().tokenize(args(vec!["orbit", "--flag", "--rate", "five", "--verbose"]));
        assert!(cli.check_option::<i32>(Optional::new("rate")).is_err());
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
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--h"]));
        let locs = cli.take_flag_locs("help");
        assert_eq!(locs.len(), 0);
        assert_eq!(cli.pull_flag(locs, false), vec![]);
    }

    #[test]
    fn check_option_n() {
        let mut cli = Cli::new().tokenize(args(vec!["orbit", "command", "--rate", "10"]));
        assert_eq!(
            cli.check_option_n(Optional::new("rate"), 1).unwrap(),
            Some(vec![10])
        );

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "command", "--rate", "10"]));
        assert_eq!(
            cli.check_option_n(Optional::new("rate"), 2).unwrap(),
            Some(vec![10])
        );

        let mut cli = Cli::new().tokenize(args(vec![
            "orbit", "command", "--rate", "10", "--rate", "4", "--rate", "3",
        ]));
        assert_eq!(
            cli.check_option_n::<u8>(Optional::new("rate"), 2).is_err(),
            true
        );
    }

    #[test]
    fn check_flag_n() {
        let mut cli = Cli::new().tokenize(args(vec!["orbit"]));
        assert_eq!(cli.check_flag_n(Flag::new("debug"), 4).unwrap(), 0);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--debug"]));
        assert_eq!(cli.check_flag_n(Flag::new("debug"), 1).unwrap(), 1);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--debug", "--debug"]));
        assert_eq!(cli.check_flag_n(Flag::new("debug"), 3).unwrap(), 2);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--debug", "--debug", "--debug"]));
        assert_eq!(cli.check_flag_n(Flag::new("debug"), 3).unwrap(), 3);

        let mut cli = Cli::new().tokenize(args(vec![
            "orbit", "--debug", "--debug", "--debug", "--debug",
        ]));
        assert_eq!(cli.check_flag_n(Flag::new("debug"), 3).is_err(), true);
    }

    #[test]
    fn check_flag_all() {
        let mut cli = Cli::new().tokenize(args(vec!["orbit"]));
        assert_eq!(cli.check_flag_all(Flag::new("debug")).unwrap(), 0);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--debug", "--debug", "--debug"]));
        assert_eq!(cli.check_flag_all(Flag::new("debug")).unwrap(), 3);

        let mut cli = Cli::new().tokenize(args(vec!["orbit", "--debug", "--debug", "--debug=1"]));
        assert_eq!(cli.check_flag_all(Flag::new("debug")).is_err(), true);
    }

    #[test]
    fn requires_positional_all() {
        let mut cli = Cli::new().tokenize(args(vec!["sum", "10", "20", "30"]));
        assert_eq!(cli.require_positional_all::<u8>(Positional::new("digit")).unwrap(), vec![10, 20, 30]);

        let mut cli = Cli::new().tokenize(args(vec!["sum"]));
        assert_eq!(cli.require_positional_all::<u8>(Positional::new("digit")).unwrap_err().kind(), ErrorKind::MissingPositional);

        let mut cli = Cli::new().tokenize(args(vec!["sum", "10", "--option", "20", "30"]));
        // note: remember to always check options and flags before positional arguments
        assert_eq!(cli.check_option::<u8>(Optional::new("option")).unwrap(), Some(20));
        assert_eq!(cli.require_positional_all::<u8>(Positional::new("digit")).unwrap(), vec![10, 30]);

        let mut cli = Cli::new().tokenize(args(vec!["sum", "100"]));
        assert_eq!(cli.require_positional_all::<u8>(Positional::new("digit")).unwrap(), vec![100]);
    }
}
