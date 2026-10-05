mod config;
mod domain;
mod fetcher;
mod generator;
mod parser;
mod ratelimit;
mod student;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use reqwest::Client;
use tokio::sync::{RwLock, mpsc};

use crate::config::AppConfig;
use crate::domain::carnet::Carnet;
use crate::fetcher::client::{FetchOutcome, fetch_student};
use crate::fetcher::fallback::try_alternate_careers;
use crate::generator::codegen::generate_all;
use crate::ratelimit::RateLimiter;
use crate::student::{Student, StudentRow};

#[derive(Default)]
struct Stats {
    found: AtomicU64,
    not_found: AtomicU64,
    transient: AtomicU64,
    skipped: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config_path, limit, only_year) = parse_args();
    let cfg = AppConfig::load(&config_path)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build()?;
    let limiter = Arc::new(RateLimiter::new(cfg.requests_per_second));

    let mut codes = generate_all(&cfg);
    if let Some(year) = only_year {
        codes.retain(|c| c.year() == year);
    }
    if let Some(limit) = limit {
        codes.truncate(limit);
    }
    println!(
        "📋 {} códigos generados | {} req/s objetivo | concurrencia {}",
        codes.len(),
        cfg.requests_per_second,
        cfg.concurrency
    );
    let legacy_count = codes.iter().filter(|c| c.is_legacy()).count();
    println!(
        "   · {} nuevos, {} viejos",
        codes.len() - legacy_count,
        legacy_count
    );
    debug_assert!(
        codes
            .iter()
            .all(|c| Carnet::parse(&c.to_string()).as_ref() == Some(c)),
        "codegen produjo carnets no parseables"
    );

    let (existing_rows, found_ids) = load_existing(&cfg.output_path);
    let found_ids = Arc::new(RwLock::new(found_ids));
    println!("♻️  {} registros previos reutilizados", existing_rows.len());

    let careers_pool = fallback_pool(&cfg);
    let (tx, rx) = mpsc::channel::<StudentRow>(2048);
    let writer = tokio::spawn(writer_task(rx, cfg.output_path.clone(), existing_rows));

    let stats = Arc::new(Stats::default());
    let start = Instant::now();
    let total = codes.len();
    let progress = tokio::spawn(progress_task(
        Arc::clone(&stats),
        Arc::clone(&limiter),
        start,
        total,
    ));

    let url = cfg.base_url.clone();
    let concurrency = cfg.concurrency.max(1);
    let max_retries = cfg.max_retries;
    let confirmations = cfg.not_found_confirmations.max(1);
    let limiter_ref = &limiter;
    let url_ref = &url;
    let tx_ref = &tx;
    let stats_ref = &stats;
    let found_ref = &found_ids;
    let careers_ref = &careers_pool;

    stream::iter(codes)
        .for_each_concurrent(concurrency, |carnet| {
            let client = client.clone();
            let tx = tx_ref.clone();
            async move {
                let key = carnet.to_string();

                if found_ref.read().await.contains(&key) {
                    stats_ref.skipped.fetch_add(1, Ordering::Relaxed);
                    return;
                }

                match fetch_student(
                    &client,
                    limiter_ref,
                    url_ref,
                    &key,
                    max_retries,
                    confirmations,
                )
                .await
                {
                    FetchOutcome::Found(student) => {
                        emit(&tx, &key, student, found_ref, stats_ref).await;
                    }
                    FetchOutcome::NotFound => {
                        match try_alternate_careers(
                            &client,
                            limiter_ref,
                            url_ref,
                            &carnet,
                            careers_ref,
                            max_retries,
                            confirmations,
                        )
                        .await
                        {
                            Some((alt, student)) => {
                                let alt_key = alt.to_string();
                                emit(&tx, &alt_key, student, found_ref, stats_ref).await;
                            }
                            None => {
                                stats_ref.not_found.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    FetchOutcome::Transient(msg) => {
                        let count = stats_ref.transient.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 100 == 1 {
                            eprintln!("⚠️  transitorio {key}: {msg}");
                        }
                    }
                }
            }
        })
        .await;

    progress.abort();
    let _ = progress.await;
    drop(tx);

    if let Err(err) = writer.await? {
        eprintln!("❌ writer: {err}");
    }

    let elapsed = start.elapsed();
    let found = stats.found.load(Ordering::Relaxed);
    let not_found = stats.not_found.load(Ordering::Relaxed);
    let transient = stats.transient.load(Ordering::Relaxed);
    let skipped = stats.skipped.load(Ordering::Relaxed);

    println!("\n✅ Completado en {:.1} min", elapsed.as_secs_f64() / 60.0);
    println!("   ✅ Encontrados: {found}");
    println!("   ❌ No encontrados: {not_found}");
    println!("   ⚠️  Transitorios (revisar): {transient}");
    println!("   ♻️  Omitidos (ya en CSV): {skipped}");
    println!(
        "   🚀 {:.1} req/s efectivas",
        (found + not_found) as f64 / elapsed.as_secs_f64().max(f64::EPSILON)
    );

    Ok(())
}

async fn emit(
    tx: &mpsc::Sender<StudentRow>,
    code: &str,
    student: Student,
    found: &RwLock<HashSet<String>>,
    stats: &Stats,
) {
    let carnet_id = student.carnet.clone().unwrap_or_else(|| code.to_string());
    {
        let mut set = found.write().await;
        set.insert(code.to_string());
        set.insert(carnet_id);
    }
    stats.found.fetch_add(1, Ordering::Relaxed);
    let row = StudentRow::from_student(code, &student);
    if tx.send(row).await.is_err() {
        eprintln!("❌ no se pudo enviar {code} al writer (canal cerrado)");
    }
}

async fn writer_task(
    mut rx: mpsc::Receiver<StudentRow>,
    path: String,
    existing: Vec<StudentRow>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&path)?;
    wtr.write_record([
        "code",
        "full_name",
        "email",
        "carnet",
        "status",
        "entry_date",
        "shift",
        "career",
    ])?;
    for row in &existing {
        wtr.serialize(row)?;
    }
    wtr.flush()?;

    let mut pending = 0u32;
    while let Some(row) = rx.recv().await {
        wtr.serialize(&row)?;
        pending += 1;
        if pending >= 200 {
            wtr.flush()?;
            pending = 0;
        }
    }
    wtr.flush()?;
    Ok(())
}

async fn progress_task(stats: Arc<Stats>, limiter: Arc<RateLimiter>, start: Instant, total: usize) {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let found = stats.found.load(Ordering::Relaxed);
        let not_found = stats.not_found.load(Ordering::Relaxed);
        let transient = stats.transient.load(Ordering::Relaxed);
        let skipped = stats.skipped.load(Ordering::Relaxed);
        let done = found + not_found + transient + skipped;
        let elapsed = start.elapsed().as_secs_f64().max(f64::EPSILON);
        println!(
            "📊 {done}/{total} | ✅{found} ❌{not_found} ⚠️{transient} ♻️{skipped} | {:.1} min | límite {:.1} req/s | {:.1} req/s real",
            start.elapsed().as_secs_f64() / 60.0,
            limiter.current_rps().await,
            done as f64 / elapsed
        );
    }
}

fn load_existing(path: &str) -> (Vec<StudentRow>, HashSet<String>) {
    let mut rows = Vec::new();
    let mut ids = HashSet::new();
    if !std::path::Path::new(path).exists() {
        return (rows, ids);
    }
    let mut reader = match csv::ReaderBuilder::new().has_headers(true).from_path(path) {
        Ok(reader) => reader,
        Err(err) => {
            eprintln!("⚠️  no se pudo leer {path} para reanudar: {err}");
            return (rows, ids);
        }
    };
    for result in reader.deserialize::<StudentRow>() {
        match result {
            Ok(row) => {
                for id in row.identifiers() {
                    ids.insert(id.to_string());
                }
                rows.push(row);
            }
            Err(err) => eprintln!("⚠️  fila inválida en {path}: {err}"),
        }
    }
    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert(row.code.clone()));
    (rows, ids)
}

fn fallback_pool(cfg: &AppConfig) -> Vec<u8> {
    let mut set = std::collections::BTreeSet::new();
    for year in &cfg.new_years {
        for &career in &year.existing_careers {
            set.insert(career);
        }
    }
    if set.is_empty() {
        for career in crate::domain::careers::CAREERS {
            set.insert(career.number);
        }
    }
    set.into_iter().collect()
}

fn parse_args() -> (String, Option<usize>, Option<u8>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config = "config.json".to_string();
    let mut limit = None;
    let mut year = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                if let Some(value) = args.get(i + 1) {
                    config = value.clone();
                    i += 1;
                }
            }
            "--limit" => {
                if let Some(value) = args.get(i + 1) {
                    limit = value.parse().ok();
                    i += 1;
                }
            }
            "--year" => {
                if let Some(value) = args.get(i + 1) {
                    year = value.parse().ok();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (config, limit, year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_pool_is_union_of_new_years() {
        let cfg = AppConfig {
            base_url: String::new(),
            output_path: String::new(),
            concurrency: 1,
            requests_per_second: 1.0,
            timeout_secs: 1,
            max_retries: 0,
            not_found_confirmations: 1,
            new_years: vec![
                crate::config::NewYearConfig {
                    year: 22,
                    start_student: 1,
                    max_students: 1,
                    existing_careers: vec![1, 3],
                },
                crate::config::NewYearConfig {
                    year: 26,
                    start_student: 1,
                    max_students: 1,
                    existing_careers: vec![3, 10],
                },
            ],
            legacy_years: vec![],
        };
        assert_eq!(fallback_pool(&cfg), vec![1, 3, 10]);
    }
}
