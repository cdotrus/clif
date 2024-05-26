use crate::arg::{Arg, Flag, Raisable};

mod tag {
    pub const FLAG: &str = "help";
    pub const SWITCH: char = 'h';
}

#[derive(Debug, PartialEq, Clone)]
pub struct Help {
    arg: Flag,
    text: String,
}

impl Help {
    pub fn new() -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::new(),
        }
    }

    pub fn with<T: AsRef<str>>(text: T) -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::from(text.as_ref()),
        }
    }

    pub fn flag<T: AsRef<str>>(mut self, name: T) -> Self {
        self.arg = Flag::new(name);
        self
    }

    pub fn switch(mut self, c: char) -> Self {
        self.arg = self.arg.switch(c);
        self
    }

    pub fn text<T: AsRef<str>>(mut self, t: T) -> Self {
        self.text = t.as_ref().to_string();
        self
    }

    pub fn get_arg(&self) -> Arg<Raisable> {
        match self.arg.get_switch() {
            Some(c) => Arg::flag(self.arg.get_name()).switch(*c),
            None => Arg::flag(self.arg.get_name()),
        }
    }

    pub fn get_text(&self) -> &str {
        self.text.as_ref()
    }
}
