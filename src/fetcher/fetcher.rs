use reqwest::Client;
use std::error::Error;
use tokio::time::sleep;
use std::time::Duration;
use crate::student::Student;
use crate::parser; // tu parser

pub async fn fetch_student(client: &Client, code: &str) -> Result<Option<Student>, Box<dyn Error>> {
    let url = "https://sive.ulsa.edu.ni/documentos/infoEstudiante";
    let mut attempts = 0;

    loop {
        attempts += 1;

        let res = client.get(url)
            .query(&[("codigo", code)])
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36 OPR/120.0.0.0 (Edition ms_store_gx)")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9,es;q=0.8")
            .header("Connection", "keep-alive")
            .header("Referer", "https://sive.ulsa.edu.ni/")
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Cookie", "_ga=GA1.1.940136856.1755286927; _ga_NGE05TRVV7=GS2.1.s1755893617$o10$g1$t1755893629$j48$l0$h0; _ga_2JNG0WPCJL=GS2.1.s1756071834$o8$g1$t1756071946$j60$l0$h0; XSRF-TOKEN=eyJpdiI6IkRPR0Q3aFlyTmFDVU1BS3NUcFk4Umc9PSIsInZhbHVlIjoibW5FY3hiSzlhM25zd1BBYWQvbTNjYWpSL0x4MnJEVGd6TFJqeHdJallsclN5bFh0SEpXSVhDMm9ZOStuWGp3UDQ2cEc3SGplMEdLTDJ3NVBwMnAzbUswNml6Z0hHUTZmbnpFTlp2ZW83OFF0WVN6NFhtQXdJRUtuMUc4MUpOcDQiLCJtYWMiOiI5M2IwYmI4NjM5ZWVhNDU3ZWM0NjU5MzVmNDZmNTM3NzViN2U5NDU3YTVjZTU1ZDcyMjgxZjU5MDk0NDc4NWUxIiwidGFnIjoiIn0%3D; laravel_session=eyJpdiI6IkdFd3RwS3JKSlZUbmRBOHYvdHJ4b0E9PSIsInZhbHVlIjoiS0JDN2wxUjVMYlFwbTJydEdoTXdMenQwa3AxcWNXRlR6alJETDRGWlZDN1k5K3ovRnM1Q1dkZlE3WmV5SVlPSzZRdVZJUlZ5UmFkcHF2SWxmQUlZaGxwUGhZQTdkc090Wk9CKzRlblBrQ3hMbW9xMFh4Ujk3d0RtMmJYeGF1bHciLCJtYWMiOiJkNzhiNGQwYmZlMjllOWQyZDBmYThhYmVkNjA5MWFkOGI5NTEwMzliNTA2N2Y1MTIwZmU1Zjk0YWU5ZGZmZWMwIiwidGFnIjoiIn0%3D")
            .send()
            .await;

        match res {
            Ok(resp) => {
                let body = resp.text().await?;
                if body.trim().is_empty() {
                    return Ok(None);
                }

                // parse_student_data devuelve Student
                let student = parser::parse_student_data(&body);

                // verifica si al menos un campo tiene datos
                let any_data = student.full_name.is_some()
                    || student.email.is_some()
                    || student.carnet.is_some()
                    || student.status.is_some()
                    || student.entry_date.is_some()
                    || student.shift.is_some()
                    || student.career.is_some();

                if any_data {
                    return Ok(Some(student));
                } else {
                    return Ok(None);
                }
            }
            Err(e) => {
                eprintln!("Error for {}: {} (attempt {})", code, e, attempts);
                if attempts >= 3 {
                    return Ok(None);
                }
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use tokio;

    #[tokio::test]
    async fn test_fetch_student_valid_code() {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Rust scraper)")
            .build()
            .unwrap();
        let code = "22-A0301-0041-A04"; // Código de prueba que debería funcionar
        let result = fetch_student(&client, code).await;
        assert!(result.is_ok(), "La petición debe ser exitosa");
        let student = result.unwrap();
        if student.is_none() {
            // Depuración: hacer la petición manual y mostrar el HTML recibido
            let url = "https://sive.ulsa.edu.ni/documentos/infoEstudiante";
            let res = client.get(url)
                .query(&[("codigo", code)])
                .header("User-Agent", "Mozilla/5.0 (Rust scraper)")
                .send()
                .await
                .unwrap();
            let body = res.text().await.unwrap();
            println!("\n--- HTML recibido para {} ---\n{}\n--- FIN HTML ---\n", code, body);
        }
        assert!(student.is_some(), "Debe encontrar datos para el código de prueba");
        let student = student.unwrap();
        println!("Student data: {:?}", student);
        assert!(student.full_name.is_some(), "El nombre completo debe estar presente");
    }

    #[tokio::test]
    async fn test_fetch_student_invalid_code() {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Rust scraper)")
            .build()
            .unwrap();
        let code = "00-XXXX-0000"; // Código inválido
        let result = fetch_student(&client, code).await;
        assert!(result.is_ok(), "La petición debe ser exitosa aunque el código no exista");
        let student = result.unwrap();
        assert!(student.is_none(), "No debe encontrar datos para un código inválido");
    }
}
