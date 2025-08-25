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
use crate::{fetcher::fetch_student, generator::config::read_year_configs};
use generator::codegen::{generate_student_codes};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let configs = read_year_configs("config.json")?;
    let student_codes = generate_student_codes(&[24, 25], &configs);

    let wtr = Writer::from_path("students.csv")?;
    let wtr = Arc::new(Mutex::new(wtr));

    stream::iter(student_codes)
        .for_each_concurrent(50, |code| {
            let client = &client;
            let wtr = Arc::clone(&wtr);

            async move {
                match fetch_student(client, &code).await {
                    Ok(Some(student)) => {
                        println!("✔ [PRUEBA] {} -> {:?}", code, student);

                        // Crear StudentRecord con referencias
                        let record = student::StudentRecord {
                            code: &code,
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
                    Ok(None) => println!("❌ No encontrado: {}", code),
                    Err(e) => eprintln!("⚠ Error para {}: {}", code, e),
                }
            }
        })
        .await;

    Ok(())
}

