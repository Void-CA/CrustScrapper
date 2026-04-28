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
use tokio::sync::Semaphore;
use std::collections::HashSet;
use tokio::sync::RwLock;

use crate::generator::config::read_year_configs;

// Cache para evitar búsquedas repetidas
struct SearchCache {
    found_codes: RwLock<HashSet<String>>,
    not_found_codes: RwLock<HashSet<String>>,
}

impl SearchCache {
    fn new() -> Self {
        Self {
            found_codes: RwLock::new(HashSet::new()),
            not_found_codes: RwLock::new(HashSet::new()),
        }
    }
    
    async fn is_found(&self, code: &str) -> Option<bool> {
        if self.found_codes.read().await.contains(code) {
            return Some(true);
        }
        if self.not_found_codes.read().await.contains(code) {
            return Some(false);
        }
        None
    }
    
    async fn mark_found(&self, code: &str) {
        self.found_codes.write().await.insert(code.to_string());
    }
    
    async fn mark_not_found(&self, code: &str) {
        self.not_found_codes.write().await.insert(code.to_string());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let configs = read_year_configs("config.json")?;
    
    // Generar códigos para varios años
    let mut all_codes = Vec::new();
    for year in [26] {
        let codes = generate_student_codes(&[year], &configs, Some(201));
        println!("📋 Año {}: {} códigos generados", year, codes.len());
        all_codes.extend(codes);
    }
    
    println!("🔍 Total de códigos a buscar: {}", all_codes.len());
    
    let wtr = Writer::from_path("students.csv")?;
    let wtr = Arc::new(Mutex::new(wtr));
    
    // Control de concurrencia
    let semaphore = Arc::new(Semaphore::new(50));
    let found_count = Arc::new(Mutex::new(0));
    let error_count = Arc::new(Mutex::new(0));
    let cache = Arc::new(SearchCache::new());
    let start_time = std::time::Instant::now();
    
    // Procesar códigos en paralelo
    stream::iter(all_codes)
        .for_each_concurrent(50, |code| {
            let client = &client;
            let wtr = Arc::clone(&wtr);
            let configs = configs.clone();
            let semaphore = Arc::clone(&semaphore);
            let found_count = Arc::clone(&found_count);
            let error_count = Arc::clone(&error_count);
            let cache = Arc::clone(&cache);
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                // Verificar cache primero
                if let Some(found) = cache.is_found(&code).await {
                    if found {
                        let mut count = found_count.lock().await;
                        *count += 1;
                    }
                    return;
                }
                
                let student_result = fetch_student(client, &code).await;
                
                match student_result {
                    Ok(Some(student)) => {
                        write_student_record(&code, &student, &wtr).await;
                        cache.mark_found(&code).await;
                        
                        let mut count = found_count.lock().await;
                        *count += 1;
                        
                        if *count % 50 == 0 {
                            let elapsed = start_time.elapsed();
                            let rate = *count as f64 / elapsed.as_secs_f64();
                            println!("📊 Progreso: {} encontrados ({:.1}/seg)", *count, rate);
                        }
                    }
                    Ok(None) => {
                        // Intentar fallback con cambio de carrera
                        if let Some(student) = try_alternate_careers(client, &code, &configs).await {
                            let carnet = student.carnet.as_ref().unwrap();
                            write_student_record(carnet, &student, &wtr).await;
                            cache.mark_found(carnet).await;
                            
                            let mut count = found_count.lock().await;
                            *count += 1;
                            println!("🔄 Encontrado (cambio carrera): {} → {}", code, carnet);
                        } else {
                            cache.mark_not_found(&code).await;
                        }
                    }
                    Err(e) => {
                        // Manejo correcto de errores reqwest
                        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
                            if reqwest_err.is_timeout() {
                                // Timeout - silencioso para no saturar
                                // println!("⏰ Timeout: {}", code);
                            } else if reqwest_err.is_connect() {
                                println!("🔌 Error conexión: {}", code);
                                let mut count = error_count.lock().await;
                                *count += 1;
                            } else if let Some(status) = reqwest_err.status() {
                                if status.is_client_error() && status != 404 {
                                    println!("⚠ HTTP {} para {}", status, code);
                                }
                            } else {
                                eprintln!("⚠ Error reqwest para {}: {}", code, reqwest_err);
                            }
                        } else {
                            eprintln!("⚠ Error inesperado para {}: {}", code, e);
                        }
                    }
                }
            }
        })
        .await;
    
    let elapsed = start_time.elapsed();
    let found = *found_count.lock().await;
    let errors = *error_count.lock().await;
    
    println!("\n✅ Completado!");
    println!("   📊 Encontrados: {} estudiantes", found);
    println!("   ⚠️  Errores de conexión: {}", errors);
    println!("   ⏱️  Tiempo total: {:.1} segundos", elapsed.as_secs_f64());
    println!("   🚀 Velocidad: {:.1} estudiantes/segundo", 
        found as f64 / elapsed.as_secs_f64());
    
    Ok(())
}

async fn write_student_record(
    code: &str, 
    student: &student::Student, 
    wtr: &Arc<Mutex<Writer<std::fs::File>>>
) {
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
    if let Err(e) = wtr.serialize(record) {
        eprintln!("❌ Error escribiendo registro para {}: {}", code, e);
    }
    if let Err(e) = wtr.flush() {
        eprintln!("❌ Error flush para {}: {}", code, e);
    }
}