use super::fetcher::fetch_student;
use crate::generator::config::YearConfig;
use reqwest::Client;

pub async fn try_alternate_careers(
    client: &Client,
    code: &str,
    configs: &[YearConfig],
) -> Option<crate::student::Student> {
    // Extraer año del código (ej: "24-...") 
    let year_prefix = &code[0..2];
    let year_num: i32 = year_prefix.parse().ok()?;

    // Buscar config del año
    let config = configs.iter().find(|c| c.year == year_num as u32)?;

    // Probar todas las carreras del año con sufijo -A0X
    for (i, _career) in config.existing_careers.iter().enumerate() {
        let alt_code = format!("{}-A{:02}", code, i + 1);
        print!("Probando código alternativo: {}", alt_code);
        if let Ok(Some(student)) = fetch_student(client, &alt_code).await {
            return Some(student.with_carnet(alt_code));
        }
    }

    None
}

