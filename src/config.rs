use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::env;
use regex::Regex;

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    pub endpoint: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rules: HashMap<String, String>,
    pub providers: HashMap<String, ProviderConfig>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        
        let re = Regex::new(r"\$\{([A-Za-z0-9_]+)\}")?;
        let mut replaced_contents = String::new();
        let mut last_match = 0;
        
        for cap in re.captures_iter(&contents) {
            let m = cap.get(0).unwrap();
            let var_name = cap.get(1).unwrap().as_str();
            
            replaced_contents.push_str(&contents[last_match..m.start()]);
            replaced_contents.push_str(&env::var(var_name).unwrap_or_default());
            
            last_match = m.end();
        }
        replaced_contents.push_str(&contents[last_match..]);
        
        let config: Config = toml::from_str(&replaced_contents)?;
        Ok(config)
    }
}
