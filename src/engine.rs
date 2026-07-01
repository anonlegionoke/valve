use regex::Regex;

pub struct Engine {
    buffer: String,
    rule: Regex,
}

impl Engine {
    pub fn new(rule_pattern: &str) -> Result<Self, regex::Error> {
        let rule = Regex::new(rule_pattern)?;
        Ok(Self {
            buffer: String::new(),
            rule,
        })
    }

    /// Appends the new token and checks if the entire buffer still matches the rule.
    /// Returns true if valid, false if the rule is violated.
    pub fn check_token(&mut self, token: &str) -> bool {
        self.buffer.push_str(token);
        self.rule.is_match(&self.buffer)
    }
}
