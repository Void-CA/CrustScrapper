use std::fmt;

use crate::domain::careers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    pub fn code(self) -> &'static str {
        match self {
            Sex::Male => "01",
            Sex::Female => "00",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "01" => Some(Sex::Male),
            "00" => Some(Sex::Female),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Carnet {
    New {
        year: u8,
        career: u8,
        sex: Sex,
        id: u16,
    },
    Changed {
        year: u8,
        career: u8,
        sex: Sex,
        id: u16,
        new_career: u8,
    },
    Legacy {
        year: u8,
        acronym: String,
        id: u16,
    },
}

impl Carnet {
    pub fn year(&self) -> u8 {
        match self {
            Carnet::New { year, .. }
            | Carnet::Changed { year, .. }
            | Carnet::Legacy { year, .. } => *year,
        }
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, Carnet::Legacy { .. })
    }

    pub fn is_changed(&self) -> bool {
        matches!(self, Carnet::Changed { .. })
    }

    #[allow(dead_code)]
    pub fn base(&self) -> Option<String> {
        match self {
            Carnet::New {
                year,
                career,
                sex,
                id,
            } => Some(format!(
                "{:02}-A{:02}{}-{:04}",
                year,
                career,
                sex.code(),
                id
            )),
            Carnet::Changed {
                year,
                career,
                sex,
                id,
                ..
            } => Some(format!(
                "{:02}-A{:02}{}-{:04}",
                year,
                career,
                sex.code(),
                id
            )),
            Carnet::Legacy { .. } => None,
        }
    }

    pub fn parse(input: &str) -> Option<Carnet> {
        let parts: Vec<&str> = input.trim().split('-').collect();
        if parts.len() < 3 || parts.len() > 4 {
            return None;
        }
        let year: u8 = parts[0].parse().ok()?;
        let id: u16 = parts[2].parse().ok()?;

        if parts.len() == 4 {
            let (career, sex) = parse_new_middle(parts[1])?;
            let new_career = parse_career_code(parts[3])?;
            return Some(Carnet::Changed {
                year,
                career,
                sex,
                id,
                new_career,
            });
        }

        if parts[1].starts_with('A') || parts[1].starts_with('a') {
            let (career, sex) = parse_new_middle(parts[1])?;
            Some(Carnet::New {
                year,
                career,
                sex,
                id,
            })
        } else {
            let acronym = parts[1].to_ascii_uppercase();
            if acronym.is_empty() || !acronym.chars().all(|c| c.is_ascii_alphabetic()) {
                return None;
            }
            careers::by_legacy_acronym(&acronym)?;
            Some(Carnet::Legacy { year, acronym, id })
        }
    }
}

impl fmt::Display for Carnet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Carnet::New {
                year,
                career,
                sex,
                id,
            } => {
                write!(f, "{:02}-A{:02}{}-{:04}", year, career, sex.code(), id)
            }
            Carnet::Changed {
                year,
                career,
                sex,
                id,
                new_career,
            } => {
                write!(
                    f,
                    "{:02}-A{:02}{}-{:04}-A{:02}",
                    year,
                    career,
                    sex.code(),
                    id,
                    new_career
                )
            }
            Carnet::Legacy { year, acronym, id } => {
                write!(f, "{:02}-{}-{:04}", year, acronym, id)
            }
        }
    }
}

fn parse_new_middle(middle: &str) -> Option<(u8, Sex)> {
    let upper = middle.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    if bytes.len() != 5 || bytes[0] != b'A' {
        return None;
    }
    let career: u8 = upper[1..3].parse().ok()?;
    careers::by_number(career)?;
    let sex = Sex::from_code(&upper[3..5])?;
    Some((career, sex))
}

fn parse_career_code(segment: &str) -> Option<u8> {
    let upper = segment.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    if bytes.len() != 3 || bytes[0] != b'A' {
        return None;
    }
    let career: u8 = upper[1..3].parse().ok()?;
    careers::by_number(career)?;
    Some(career)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_new_format() {
        let c = Carnet::parse("26-A0201-0237").unwrap();
        assert_eq!(
            c,
            Carnet::New {
                year: 26,
                career: 2,
                sex: Sex::Male,
                id: 237
            }
        );
        assert_eq!(c.to_string(), "26-A0201-0237");
        assert_eq!(c.base().unwrap(), "26-A0201-0237");
        assert!(!c.is_legacy());
    }

    #[test]
    fn parses_female_new_format() {
        let c = Carnet::parse("22-A0300-0001").unwrap();
        assert_eq!(
            c,
            Carnet::New {
                year: 22,
                career: 3,
                sex: Sex::Female,
                id: 1
            }
        );
    }

    #[test]
    fn parses_changed_format() {
        let c = Carnet::parse("22-A0301-0041-A04").unwrap();
        assert_eq!(
            c,
            Carnet::Changed {
                year: 22,
                career: 3,
                sex: Sex::Male,
                id: 41,
                new_career: 4
            }
        );
        assert_eq!(c.to_string(), "22-A0301-0041-A04");
        assert_eq!(c.base().unwrap(), "22-A0301-0041");
        assert!(c.is_changed());
    }

    #[test]
    fn parses_legacy_format() {
        let c = Carnet::parse("18-IME-0053").unwrap();
        assert_eq!(
            c,
            Carnet::Legacy {
                year: 18,
                acronym: "IME".into(),
                id: 53
            }
        );
        assert_eq!(c.to_string(), "18-IME-0053");
        assert!(c.base().is_none());
        assert!(c.is_legacy());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Carnet::parse("").is_none());
        assert!(Carnet::parse("00-XXXX-0000").is_none());
        assert!(Carnet::parse("26-A0199-0001").is_none());
        assert!(Carnet::parse("26-A0202-0001").is_none());
        assert!(Carnet::parse("26-A0201-0001-A99").is_none());
    }

    #[test]
    fn round_trips() {
        for s in ["26-A0700-0236", "22-A0301-0041-A04", "17-IMS-0031"] {
            assert_eq!(Carnet::parse(s).unwrap().to_string(), s);
        }
    }
}
