use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YearConfig {
    pub year: u32,
    pub max_students: u32,
    pub existing_careers: Vec<u8>,
}


pub fn read_year_configs(path: &str) -> Result<Vec<YearConfig>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let configs: Vec<YearConfig> = serde_json::from_str(&content)?;
    Ok(configs)
}

pub fn write_year_configs(path: &str, configs: &[YearConfig]) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(configs)?;
    fs::write(path, content)?;
    Ok(())
}
