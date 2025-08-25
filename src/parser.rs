use scraper::{Html, Selector};
use crate::student::Student;

/// Dado un <div> con <strong>Campo:</strong>, devuelve el valor del siguiente div con clase col-md-12
fn extract_value_strong(div: scraper::ElementRef, document: &Html) -> Option<String> {
    let mut found = false;
    for row in document.select(&Selector::parse("div.row.mt-2").unwrap()) {
        if found {
            // El siguiente div después del <strong> es el valor
            return Some(row.text().collect::<String>().trim().to_string());
        }
        if let Some(strong) = row.select(&Selector::parse("strong").unwrap()).next() {
            if strong.inner_html().trim_end_matches(':') == div.inner_html().trim_end_matches(':') {
                found = true;
            }
        }
    }
    None
}

pub fn parse_student_data(html: &str) -> Student {
    let document = Html::parse_document(html);

    // Para nombres y apellidos
    let mut full_name = None;
    let nombre_divs: Vec<_> = document.select(&Selector::parse("strong").unwrap())
        .filter(|s| {
            let binding = s.inner_html();
            let text = binding.trim_end_matches(':');
            text == "Nombres" || text == "Apellidos"
        })
        .collect();

    if !nombre_divs.is_empty() {
        let nombres = extract_value_strong(nombre_divs[0], &document).unwrap_or_default();
        let apellidos = extract_value_strong(nombre_divs[1], &document).unwrap_or_default();
        full_name = Some(format!("{} {}", nombres, apellidos));
    }

    // Otros campos
    let carnet = document.select(&Selector::parse("strong").unwrap())
        .find(|s| s.inner_html().trim_end_matches(':') == "Carnet")
        .and_then(|d| extract_value_strong(d, &document));

    let turno = document.select(&Selector::parse("strong").unwrap())
        .find(|s| s.inner_html().trim_end_matches(':') == "Turno")
        .and_then(|d| extract_value_strong(d, &document));

    let estado = document.select(&Selector::parse("strong").unwrap())
        .find(|s| s.inner_html().trim_end_matches(':') == "Estado")
        .and_then(|d| extract_value_strong(d, &document));

    // Selector para email
    let email_selector = scraper::Selector::parse("a[href^='mailto:']").unwrap();
    let email = document
        .select(&email_selector)
        .next()
        .and_then(|n| n.value().attr("href"))
        .map(|h| h.trim_start_matches("mailto:").to_string());

    // Programa/Carrera y Career
    let carrera = document.select(&Selector::parse("strong").unwrap())
        .find(|s| s.inner_html().trim_end_matches(':') == "Programa/Carrera")
        .and_then(|d| extract_value_strong(d, &document));

    let entry_date = document.select(&Selector::parse("strong").unwrap())
        .find(|s| s.inner_html().trim_end_matches(':') == "Fecha de ingreso")
        .and_then(|d| extract_value_strong(d, &document));


    Student {
        full_name,
        email,
        carnet,
        status: estado,
        entry_date: entry_date,
        shift: turno,
        career: carrera,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_student_data_full() {
        let html = r#"
        <div class='row mt-2'><strong>Nombres:</strong></div>
        <div class='row mt-2'>ARI ALEJANDRO</div>
        <div class='row mt-2'><strong>Apellidos:</strong></div>
        <div class='row mt-2'>CASTILLO AMADOR</div>
        <div class='row mt-2'><strong>Carnet:</strong></div>
        <div class='row mt-2'>22-A0301-0041-A04</div>
        <div class='row mt-2'><strong>Turno:</strong></div>
        <div class='row mt-2'>Diurno</div>
        <div class='row mt-2'><strong>Estado:</strong></div>
        <div class='row mt-2'>ACTIVO</div>
        <div class='row mt-2'><strong>Programa/Carrera:</strong></div>
        <div class='row mt-2'>Ingeniería en Cibernética Electrónica</div>
        <div class='row mt-2'><strong>Fecha de ingreso:</strong></div>
        <div class='row mt-2'>16/12/2022</div>
        <a href='mailto:ari.castillo@est.ulsa.edu.ni'>ari.castillo@est.ulsa.edu.ni</a>
        "#;
        let student = parse_student_data(html);
        assert_eq!(student.full_name, Some("ARI ALEJANDRO CASTILLO AMADOR".to_string()));
        assert_eq!(student.email, Some("ari.castillo@est.ulsa.edu.ni".to_string()));
        assert_eq!(student.carnet, Some("22-A0301-0041-A04".to_string()));
        assert_eq!(student.shift, Some("Diurno".to_string()));
        assert_eq!(student.status, Some("ACTIVO".to_string()));
        assert_eq!(student.career, Some("Ingeniería en Cibernética Electrónica".to_string()));
        assert_eq!(student.entry_date, Some("16/12/2022".to_string()));
    }

    #[test]
    fn test_parse_student_data_partial() {
        let html = r#"
        <div class='row mt-2'><strong>Nombres:</strong></div>
        <div class='row mt-2'>ARI</div>
        <div class='row mt-2'><strong>Apellidos:</strong></div>
        <div class='row mt-2'>CASTILLO</div>
        <a href='mailto:ari.castillo@est.ulsa.edu.ni'>ari.castillo@est.ulsa.edu.ni</a>
        "#;
        let student = parse_student_data(html);
        assert_eq!(student.full_name, Some("ARI CASTILLO".to_string()));
        assert_eq!(student.email, Some("ari.castillo@est.ulsa.edu.ni".to_string()));
        assert!(student.carnet.is_none());
        assert!(student.shift.is_none());
        assert!(student.status.is_none());
        assert!(student.career.is_none());
        assert!(student.entry_date.is_none());
    }

    #[test]
    fn test_parse_student_data_empty() {
        let html = "";
        let student = parse_student_data(html);
        assert!(student.full_name.is_none());
        assert!(student.email.is_none());
        assert!(student.carnet.is_none());
        assert!(student.shift.is_none());
        assert!(student.status.is_none());
        assert!(student.career.is_none());
        assert!(student.entry_date.is_none());
    }
}