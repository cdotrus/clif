use crate::arg::{Arg, Flag, Raisable};

mod tag {
    pub const FLAG: &str = "help";
    pub const SWITCH: char = 'h';
}

/// A special flag that can have priority over other arguments in command-line
/// processing.
#[derive(Debug, PartialEq, Clone)]
pub struct Help {
    arg: Flag,
    text: String,
}

impl Help {
    /// Create a new [Help] flag.
    ///
    /// By default, it sets the flag's name to "help" and the flag's switch to "h".
    /// To use a custom flag, use [flag][Help::flag].
    pub fn new() -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::new(),
        }
    }

    /// Create a new [Help] flag with informational text `text`.
    ///
    /// By default, it sets the flag's name to "help" and the flag's switch to "h".
    /// To use a custom flag, use [flag][Help::flag].
    pub fn with<T: AsRef<str>>(text: T) -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::from(text.as_ref()),
        }
    }

    /// Set the [Help] flag's name to `name`.
    ///
    /// Once this is set, any previous switch is removed. To add a switch, use
    /// [switch][Help::switch].
    pub fn flag<T: AsRef<str>>(mut self, name: T) -> Self {
        self.arg = Flag::new(name);
        self
    }

    /// Set the [Help] flag's switch to `c`.
    pub fn switch(mut self, c: char) -> Self {
        self.arg = self.arg.switch(c);
        self
    }

    /// Set the [Help] flag's informational text to `t`.
    pub fn text<T: AsRef<str>>(mut self, t: T) -> Self {
        self.text = t.as_ref().to_string();
        self
    }

    /// Transform the [Help] flag into its [Arg].
    pub fn get_arg(&self) -> Arg<Raisable> {
        match self.arg.get_switch() {
            Some(c) => Arg::flag(self.arg.get_name()).switch(*c),
            None => Arg::flag(self.arg.get_name()),
        }
    }

    /// Access the [Help] flag's informational text.
    pub fn get_text(&self) -> &str {
        self.text.as_ref()
    }
}
