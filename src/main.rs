mod student;
mod fetcher;
mod generator;
mod parser;

use tokio::sync::Mutex;
use std::sync::Arc;
use csv::Writer;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use tokio::time::Duration;
use fetcher::{fetcher::fetch_student, fallback::try_alternate_careers};
use generator::codegen::{generate_student_codes};

use crate::generator::config::read_year_configs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let configs = read_year_configs("config.json")?;
    let mut student_codes = generate_student_codes(&[25], &configs, Some(1));
    student_codes.push("22-A0301-0041".to_string());
    let wtr = Writer::from_path("students.csv")?;
    let wtr = Arc::new(Mutex::new(wtr));

    stream::iter(student_codes)
        .for_each_concurrent(100, |code| {
            let client = &client;
            let wtr = Arc::clone(&wtr);
            let configs = configs.clone();

            async move {
                // Intento 1: código normal de 3 segmentos
                let student_result = fetch_student(client, &code).await;

                match student_result {
                    Ok(Some(student)) => {
                        write_student_record(&code, &student, &wtr).await;
                        println!("✔ Encontrado: {}", code);
                    }
                    Ok(None) => {
                        // Si no se encuentra, intento fallback con cambio de carrera
                        if let Some(student) = try_alternate_careers(client, &code, &configs).await {
                            write_student_record(&student.carnet.as_ref().unwrap(), &student, &wtr).await;
                            println!("✔ Encontrado (carrera alternativa): {}", student.carnet.as_ref().unwrap());
                        } else {
                            println!("❌ No encontrado: {}", code);
                        }
                    }
                    Err(e) => eprintln!("⚠ Error para {}: {}", code, e),
                }
            }
        })
        .await;


    Ok(())
}

// Encapsula la lógica de creación y escritura de StudentRecord
async fn write_student_record(code: &str, student: &student::Student, wtr: &Arc<Mutex<Writer<std::fs::File>>>) {
    let record = student::StudentRecord {
        code,
        full_name: student.full_name.as_deref(),
        email: student.email.as_deref(),
        carnet: student.carnet.as_deref(),
        status: student.status.as_deref(),
        entry_date: student.entry_date.as_deref(),
        shift: student.shift.as_deref(),
        career: student.career.as_deref(),
    };

    let mut wtr = wtr.lock().await;
    wtr.serialize(record).unwrap();
    wtr.flush().unwrap();
}

