use crate::arg::Flag;
use std::ops::Range;

mod tag {
    pub const FLAG: &str = "help";
    pub const SWITCH: char = 'h';
}

#[derive(Debug, PartialEq, Clone)]
pub struct Help {
    arg: Flag,
    summary: Option<String>,
    usage: Option<String>,
    quick_text: String,
    long_text: Option<String>,
}

impl Help {
    pub fn new() -> Self {
        Self {
            arg: Flag::new(tag::FLAG).switch(tag::SWITCH),
            summary: None,
            usage: None,
            quick_text: String::new(),
            long_text: None,
        }
    }

    pub fn long_text<T: AsRef<str>>(mut self, t: T) -> Self {
        self.long_text = Some(t.as_ref().to_string());
        self
    }

    pub fn flag(mut self, f: Flag) -> Self {
        self.arg = f;
        self
    }

    pub fn quick_text<T: AsRef<str>>(mut self, t: T) -> Self {
        self.quick_text = t.as_ref().to_string();
        self
    }

    pub fn usage<T: AsRef<str>>(mut self, t: T) -> Self {
        self.usage = Some(t.as_ref().to_string());
        self
    }

    pub fn get_flag(&self) -> &Flag {
        &self.arg
    }

    pub fn get_quick_text(&self) -> &str {
        self.quick_text.as_ref()
    }

    pub fn get_usage(&self) -> Option<&str> {
        Some(self.usage.as_ref()?.as_ref())
    }

    /// References the appropriate lines for a text statement for usage according to the line range `line_bounds`.
    ///
    /// The function will fail to set a usage statement without panicking if the the range is out-of-bounds.
    ///
    /// The range must be specified as `inclusive..exclusive`.
    pub fn ref_usage(mut self, line_bounds: Range<usize>) -> Self {
        let mut lines = self.get_quick_text().split_terminator('\n').enumerate();
        // find the starting character
        let mut start_char: Option<usize> = None;
        // find the ending character
        let mut end_char: Option<usize> = None;

        let mut running_sum: usize = 0;

        while let Some((i, data)) = lines.next() {
            if i == line_bounds.start {
                start_char = Some(running_sum);
            }

            // increment the number of characters passed (+1 to account for missing NEW_LINE character that was split)
            running_sum += data.len() + 1;

            // check if we hit the upper bound of the index (inclusive)
            if i + 1 == line_bounds.end {
                end_char = Some(running_sum - 1);
                break;
            }
        }
        // ensure there is a starting and ending character index to reference.
        if start_char.is_some() == true && end_char.is_some() == true {
            self.usage = Some(
                self.quick_text
                    .get(start_char.unwrap()..end_char.unwrap())
                    .unwrap()
                    .to_owned(),
            );
        }

        self
    }
}
