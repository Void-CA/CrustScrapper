#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Career {
    pub number: u8,
    pub code: &'static str,
    pub name: &'static str,
    pub legacy_acronyms: &'static [&'static str],
}

pub const CAREERS: &[Career] = &[
    Career {
        number: 1,
        code: "A01",
        name: "Ingeniería en Mecánica y Energías Renovables",
        legacy_acronyms: &["IME"],
    },
    Career {
        number: 2,
        code: "A02",
        name: "Ingeniería en Mecatrónica y Sistemas de Control",
        legacy_acronyms: &["IMS"],
    },
    Career {
        number: 3,
        code: "A03",
        name: "Ingeniería en Gestión Industrial",
        legacy_acronyms: &["IGI"],
    },
    Career {
        number: 4,
        code: "A04",
        name: "Ingeniería en Cibernética Electrónica",
        legacy_acronyms: &["ICE"],
    },
    Career {
        number: 5,
        code: "A05",
        name: "Ingeniería Eléctrica con énfasis en Eficiencia Energética",
        legacy_acronyms: &["IEEE"],
    },
    Career {
        number: 6,
        code: "A06",
        name: "Carrera desconocida (A06)",
        legacy_acronyms: &[],
    },
    Career {
        number: 7,
        code: "A07",
        name: "Licenciatura Comercial con énfasis en Mercadeo",
        legacy_acronyms: &[],
    },
    Career {
        number: 8,
        code: "A08",
        name: "Licenciatura en Administración con énfasis en Finanzas",
        legacy_acronyms: &[],
    },
    Career {
        number: 9,
        code: "A09",
        name: "Carrera desconocida (A09)",
        legacy_acronyms: &[],
    },
    Career {
        number: 10,
        code: "A10",
        name: "Ingeniería Electromédica",
        legacy_acronyms: &[],
    },
];

pub fn by_number(number: u8) -> Option<&'static Career> {
    CAREERS.iter().find(|c| c.number == number)
}

#[allow(dead_code)]
pub fn by_code(code: &str) -> Option<&'static Career> {
    let code = code.to_ascii_uppercase();
    CAREERS.iter().find(|c| c.code == code)
}

pub fn by_legacy_acronym(acronym: &str) -> Option<&'static Career> {
    let acronym = acronym.to_ascii_uppercase();
    CAREERS.iter().find(|c| {
        c.legacy_acronyms
            .iter()
            .any(|a| a.eq_ignore_ascii_case(&acronym))
    })
}

#[allow(dead_code)]
pub fn code_for(number: u8) -> Option<&'static str> {
    by_number(number).map(|c| c.code)
}

#[allow(dead_code)]
pub fn name_for(number: u8) -> Option<&'static str> {
    by_number(number).map(|c| c.name)
}

#[allow(dead_code)]
pub fn legacy_acronyms() -> Vec<&'static str> {
    CAREERS
        .iter()
        .flat_map(|c| c.legacy_acronyms.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_new_and_legacy() {
        assert_eq!(
            by_code("a03").unwrap().name,
            "Ingeniería en Gestión Industrial"
        );
        assert_eq!(by_legacy_acronym("ime").unwrap().number, 1);
        assert_eq!(by_legacy_acronym("IEEE").unwrap().number, 5);
    }

    #[test]
    fn acronyms_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for c in CAREERS {
            for a in c.legacy_acronyms {
                assert!(seen.insert(*a), "acrónimo duplicado: {a}");
            }
        }
    }
}
