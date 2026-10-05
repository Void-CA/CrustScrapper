use std::collections::HashSet;

use crate::config::{AppConfig, LegacyYearConfig, NewYearConfig};
use crate::domain::careers;
use crate::domain::carnet::{Carnet, Sex};

pub fn sex_order(career: u8) -> [Sex; 2] {
    if (1..=6).contains(&career) {
        [Sex::Male, Sex::Female]
    } else {
        [Sex::Female, Sex::Male]
    }
}

pub fn generate_new_codes(years: &[NewYearConfig]) -> Vec<Carnet> {
    let mut out = Vec::new();
    for cfg in years {
        if cfg.start_student == 0 || cfg.max_students < cfg.start_student {
            continue;
        }
        for id in cfg.start_student..=cfg.max_students {
            for &career in &cfg.existing_careers {
                if careers::by_number(career).is_none() {
                    continue;
                }
                for sex in sex_order(career) {
                    out.push(Carnet::New {
                        year: cfg.year,
                        career,
                        sex,
                        id,
                    });
                }
            }
        }
    }
    out
}

pub fn generate_legacy_codes(years: &[LegacyYearConfig]) -> Vec<Carnet> {
    let mut out = Vec::new();
    for cfg in years {
        if cfg.start_student == 0 || cfg.max_students < cfg.start_student {
            continue;
        }
        for id in cfg.start_student..=cfg.max_students {
            for acronym in &cfg.acronyms {
                let acronym = acronym.to_ascii_uppercase();
                if careers::by_legacy_acronym(&acronym).is_none() {
                    continue;
                }
                out.push(Carnet::Legacy {
                    year: cfg.year,
                    acronym,
                    id,
                });
            }
        }
    }
    out
}

pub fn generate_all(cfg: &AppConfig) -> Vec<Carnet> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for carnet in generate_legacy_codes(&cfg.legacy_years)
        .into_iter()
        .chain(generate_new_codes(&cfg.new_years))
    {
        if seen.insert(carnet.to_string()) {
            out.push(carnet);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_year(year: u8, max: u16, careers: Vec<u8>) -> NewYearConfig {
        NewYearConfig {
            year,
            start_student: 1,
            max_students: max,
            existing_careers: careers,
        }
    }

    #[test]
    fn new_codes_have_expected_shape_and_order() {
        let codes = generate_new_codes(&[new_year(26, 3, vec![1, 8])]);
        assert_eq!(codes.len(), 3 * 2 * 2);
        assert_eq!(codes[0].to_string(), "26-A0101-0001");
        assert_eq!(codes[1].to_string(), "26-A0100-0001");
        let first_a08 = codes
            .iter()
            .find(|c| c.to_string().starts_with("26-A08"))
            .unwrap();
        assert_eq!(first_a08.to_string(), "26-A0800-0001");
        assert_eq!(
            codes
                .iter()
                .map(|c| c.to_string())
                .collect::<HashSet<_>>()
                .len(),
            codes.len()
        );
    }

    #[test]
    fn legacy_codes_are_generated_and_validated() {
        let codes = generate_legacy_codes(&[LegacyYearConfig {
            year: 18,
            start_student: 1,
            max_students: 2,
            acronyms: vec!["IME".into(), "ZZZ".into()],
        }]);
        assert_eq!(
            codes.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            vec!["18-IME-0001", "18-IME-0002"]
        );
    }

    #[test]
    fn generate_all_dedups_and_covers_both_formats() {
        let cfg = AppConfig {
            base_url: String::new(),
            output_path: String::new(),
            concurrency: 1,
            requests_per_second: 1.0,
            timeout_secs: 1,
            max_retries: 0,
            not_found_confirmations: 1,
            new_years: vec![new_year(26, 2, vec![3]), new_year(26, 2, vec![3])],
            legacy_years: vec![LegacyYearConfig {
                year: 18,
                start_student: 1,
                max_students: 1,
                acronyms: vec!["ICE".into()],
            }],
        };
        let all = generate_all(&cfg);
        assert_eq!(all.iter().filter(|c| c.is_legacy()).count(), 1);
        assert_eq!(all.iter().filter(|c| !c.is_legacy()).count(), 4);
        assert_eq!(
            all.iter()
                .map(|c| c.to_string())
                .collect::<HashSet<_>>()
                .len(),
            all.len()
        );
    }
}
