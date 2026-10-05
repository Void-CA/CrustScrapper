use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewYearConfig {
    pub year: u8,
    #[serde(default = "default_start")]
    pub start_student: u16,
    pub max_students: u16,
    pub existing_careers: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LegacyYearConfig {
    pub year: u8,
    #[serde(default = "default_start")]
    pub start_student: u16,
    pub max_students: u16,
    pub acronyms: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_output")]
    pub output_path: String,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_rps")]
    pub requests_per_second: f64,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    #[serde(default = "default_confirmations")]
    pub not_found_confirmations: u32,
    #[serde(default = "default_true")]
    pub fallback_enabled: bool,
    #[serde(default)]
    pub new_years: Vec<NewYearConfig>,
    #[serde(default)]
    pub legacy_years: Vec<LegacyYearConfig>,
}

fn default_start() -> u16 {
    1
}

fn default_base_url() -> String {
    "https://sive.ulsa.edu.ni/documentos/infoEstudiante".to_string()
}

fn default_output() -> String {
    "students.csv".to_string()
}

fn default_concurrency() -> usize {
    12
}

fn default_rps() -> f64 {
    8.0
}

fn default_timeout() -> u64 {
    8
}

fn default_retries() -> u32 {
    4
}

fn default_confirmations() -> u32 {
    2
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    pub fn load(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn new_year(&self, year: u8) -> Option<&NewYearConfig> {
        self.new_years.iter().find(|c| c.year == year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config_with_defaults() {
        let json = r#"{
            "new_years": [{ "year": 26, "max_students": 10, "existing_careers": [1, 3] }],
            "legacy_years": [{ "year": 18, "max_students": 5, "acronyms": ["IME"] }]
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.concurrency, default_concurrency());
        assert_eq!(cfg.not_found_confirmations, default_confirmations());
        assert_eq!(cfg.new_years[0].start_student, 1);
        assert_eq!(cfg.legacy_years[0].acronyms, vec!["IME".to_string()]);
    }
}
