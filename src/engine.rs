use regex::Regex;
use serde_json::Value;
use jsonschema::Validator;
use crate::config::RuleConfig;

pub enum Engine {
    Regex {
        buffer: String,
        rule: Regex,
    },
    Json {
        buffer: String,
        validator: Validator,
    }
}

impl Engine {
    pub fn new(config: &RuleConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match config {
            RuleConfig::Regex { regex } => {
                let rule = Regex::new(regex)?;
                Ok(Self::Regex {
                    buffer: String::new(),
                    rule,
                })
            }
            RuleConfig::Schema { schema } => {
                let validator = jsonschema::validator_for(schema)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                Ok(Self::Json {
                    buffer: String::new(),
                    validator,
                })
            }
        }
    }

    pub fn check_token(&mut self, token: &str) -> bool {
        match self {
            Self::Regex { buffer, rule } => {
                buffer.push_str(token);
                rule.is_match(buffer)
            }
            Self::Json { buffer, validator } => {
                buffer.push_str(token);
                
                match serde_json::from_str::<Value>(buffer) {
                    Ok(val) => {
                        validator.is_valid(&val)
                    }
                    Err(e) => {
                        e.is_eof()
                    }
                }
            }
        }
    }

    pub fn pop_token(&mut self, token: &str) {
        match self {
            Self::Regex { buffer, .. } | Self::Json { buffer, .. } => {
                let new_len = buffer.len().saturating_sub(token.len());
                buffer.truncate(new_len);
            }
        }
    }
}
