use std::fmt::Display;
use std::fmt::Debug;

mod symbol {
    pub const FLAG: &str = "--";
    pub const POS_BRACKET_L: &str = "<";
    pub const POS_BRACKER_R: &str = ">";
}

#[derive(PartialEq)]
pub enum Arg<'a> {
    Flag(Flag<'a>),
    Positional(Positional<'a>),
    Optional(Optional<'a>),
}

impl<'a> Arg<'a> {
    pub fn as_flag(&self) -> Option<&Flag> {
        match self {
            Arg::Flag(f) => Some(f),
            Arg::Optional(o) => Some(o.get_flag()),
            Arg::Positional(_) => None,
        }
    }
}

impl<'a> Display for Arg<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Arg::Flag(a) => write!(f, "{}", a),
            Arg::Positional(a) => write!(f, "{}", a),
            Arg::Optional(a) => write!(f, "{}", a),
        }
    }
}

impl<'a> Debug for Arg<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct Positional<'a> {
    name: &'a str,
}

impl<'a> Positional<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { name: s }
    }
}

impl<'a> Display for Positional<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}{}{}",
            symbol::POS_BRACKET_L,
            self.name,
            symbol::POS_BRACKER_R
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Flag<'a> {
    name: &'a str,
    switch: Option<char>,
}

impl<'a> Flag<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            name: s,
            switch: None,
        }
    }

    pub fn switch(mut self, c: char) -> Self {
        self.switch = Some(c);
        self
    }

    pub fn get_name(&self) -> &str {
        self.name
    }

    pub fn get_switch(&self) -> Option<&char> {
        self.switch.as_ref()
    }
}

impl<'a> Display for Flag<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}{}", symbol::FLAG, self.name)
    }
}

#[derive(Debug, PartialEq)]
pub struct Optional<'a> {
    option: Flag<'a>,
    value: Positional<'a>,
}

impl<'a> Optional<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            option: Flag::new(s),
            value: Positional::new(s),
        }
    }

    pub fn value(mut self, s: &'a str) -> Self {
        self.value.name = s;
        self
    }

    pub fn switch(mut self, c: char) -> Self {
        self.option.switch = Some(c);
        self
    }

    pub fn get_flag(&self) -> &Flag {
        &self.option
    }

    pub fn _get_pos(&self) -> &Positional {
        &self.value
    }
}

impl<'a> Display for Optional<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} {}", self.option, self.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn positional_new() {
        let ip = Positional::new("ip");
        assert_eq!(ip, Positional { name: "ip" });

        let version = Positional::new("version");
        assert_eq!(version, Positional { name: "version" });
    }

    #[test]
    fn positional_disp() {
        let ip = Positional::new("ip");
        assert_eq!(ip.to_string(), "<ip>");

        let topic = Positional::new("topic");
        assert_eq!(topic.to_string(), "<topic>");
    }

    #[test]
    fn flag_new() {
        let help = Flag::new("help").switch('h');
        assert_eq!(
            help,
            Flag {
                name: "help",
                switch: Some('h'),
            }
        );
        assert_eq!(help.get_switch(), Some(&'h'));
        assert_eq!(help.get_name(), "help");

        let version = Flag::new("version");
        assert_eq!(
            version,
            Flag {
                name: "version",
                switch: None,
            }
        );
        assert_eq!(version.get_switch(), None);
        assert_eq!(version.get_name(), "version");
    }

    #[test]
    fn flag_disp() {
        let help = Flag::new("help");
        assert_eq!(help.to_string(), "--help");

        let version = Flag::new("version");
        assert_eq!(version.to_string(), "--version");
    }

    #[test]
    fn optional_new() {
        let code = Optional::new("code");
        assert_eq!(
            code,
            Optional {
                option: Flag::new("code"),
                value: Positional::new("code"),
            }
        );
        assert_eq!(code.get_flag().get_switch(), None);

        let version = Optional::new("color").value("rgb");
        assert_eq!(
            version,
            Optional {
                option: Flag::new("color"),
                value: Positional::new("rgb"),
            }
        );
        assert_eq!(version.get_flag().get_switch(), None);

        let version = Optional::new("color").value("rgb").switch('c');
        assert_eq!(
            version,
            Optional {
                option: Flag::new("color").switch('c'),
                value: Positional::new("rgb"),
            }
        );
        assert_eq!(version.get_flag().get_switch(), Some(&'c'));

        assert_eq!(version._get_pos(), &Positional::new("rgb"));
    }

    #[test]
    fn optional_disp() {
        let code = Optional::new("code");
        assert_eq!(code.to_string(), "--code <code>");

        let color = Optional::new("color").value("rgb");
        assert_eq!(color.to_string(), "--color <rgb>");

        let color = Optional::new("color").value("rgb").switch('c');
        assert_eq!(color.to_string(), "--color <rgb>");
    }

    #[test]
    fn arg_disp() {
        let command = Arg::Positional(Positional::new("command"));
        assert_eq!(command.to_string(), "<command>");

        let help = Arg::Flag(Flag::new("help"));
        assert_eq!(help.to_string(), "--help");

        assert_eq!(help.as_flag().unwrap().to_string(), "--help");

        let color = Arg::Optional(Optional::new("color").value("rgb"));
        assert_eq!(color.to_string(), "--color <rgb>");

        assert_eq!(color.as_flag().unwrap().get_name(), "color");
    }

    #[test]
    fn arg_impossible_pos_as_flag() {
        let command = Arg::Positional(Positional::new("command"));
        assert_eq!(command.as_flag(), None);
    }
}
