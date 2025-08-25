
use crate::generator::config::YearConfig;
pub fn generate_student_codes(years: &[u32], configs: &[YearConfig], starting_number: Option<u32>) -> Vec<String> {
    let mut student_codes = Vec::new();

    let start_num = starting_number.unwrap_or(1);

    for &year in years {
        if let Some(config) = configs.iter().find(|c| c.year == year) {
            for student_num in start_num..=config.max_students {
                for &carrera_num in &config.existing_careers {
                    let carrera = format!("A{:02}", carrera_num);
                    let sex_order = if (1..=6).contains(&carrera_num) { vec!["01", "00"] } else { vec!["00", "01"] };

                    for sexo in sex_order {
                        student_codes.push(format!("{:02}-{}{}-{:04}", config.year, carrera, sexo, student_num));
                    }
                }
            }
        }
    }

    student_codes
}





#[cfg(test)]
mod tests {
    use crate::generator::config::read_year_configs;

    use super::*;

    #[test]
    fn test_generate_student_codes_basic() {
        let codes = generate_student_codes(&[22], &read_year_configs("config.json").unwrap(), None);
        // Debe generar códigos
        assert!(!codes.is_empty(), "Debe generar al menos un código");
        // Debe tener el formato correcto
        let sample = &codes[0];
        assert!(sample.starts_with("22-A"), "El código debe empezar con año y carrera");
        assert_eq!(sample.len(), 13, "El código base debe tener longitud 13");
    }

    #[test]
    fn test_generate_student_codes_no_duplicates() {
        let codes = generate_student_codes(&[22], &read_year_configs("config.json").unwrap(), None);
        let mut set = std::collections::HashSet::new();
        for code in codes {
            assert!(set.insert(code), "No debe haber códigos duplicados");
        }
    }


    #[test]
    fn test_generate_student_codes_sex_order() {
        let codes = generate_student_codes(&[22], &read_year_configs("config.json").unwrap(), None);

        // Carrera 1 debería empezar con "01"
        let first_c1 = codes.iter().find(|c| c.starts_with("22-A01")).unwrap();
        assert!(first_c1.contains("01-0001"));

        // Carrera 7 debería empezar con "00"
        let first_c7 = codes.iter().find(|c| c.starts_with("22-A08")).unwrap();
        assert!(first_c7.contains("00-0001"));
    }
}
