use crate::arg::Flag;

mod tag {
    pub const FLAG: &str = "help";
    pub const SWITCH: char = 'h';
}

#[derive(Debug, PartialEq, Clone)]
pub struct Help {
    arg: Flag,
    text: String,
}

impl Default for Help {
    fn default() -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::default(),
        }
    }
}

impl Help {
    pub fn new() -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            text: String::new(),
        }
    }

    pub fn flag(mut self, f: Flag) -> Self {
        self.arg = f;
        self
    }

    pub fn text<T: AsRef<str>>(mut self, t: T) -> Self {
        self.text = t.as_ref().to_string();
        self
    }

    pub fn get_flag(&self) -> &Flag {
        &self.arg
    }

    pub fn get_text(&self) -> &str {
        self.text.as_ref()
    }
}
