use std::time::Duration;

use reqwest::Client;
use reqwest::header::RETRY_AFTER;
use tokio::time::sleep;

use crate::parser;
use crate::ratelimit::RateLimiter;
use crate::student::Student;

#[derive(Debug)]
pub enum FetchOutcome {
    Found(Student),
    NotFound,
    Transient(String),
}

#[derive(Debug)]
pub enum BodyKind {
    Data(Student),
    NotFound,
    Unknown,
}

pub fn classify_body(body: &str) -> BodyKind {
    if body.contains("no se ha encontrado ningún estudiante") {
        return BodyKind::NotFound;
    }
    let student = parser::parse_student_data(body);
    if student.has_data() {
        BodyKind::Data(student)
    } else {
        BodyKind::Unknown
    }
}

pub async fn fetch_student(
    client: &Client,
    limiter: &RateLimiter,
    url: &str,
    code: &str,
    max_retries: u32,
    not_found_confirmations: u32,
) -> FetchOutcome {
    let mut not_found_seen = 0u32;
    let confirmations = not_found_confirmations.max(1);

    for attempt in 0..=max_retries {
        limiter.acquire().await;

        let response = client
            .get(url)
            .query(&[("codigo", code)])
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "es-NI,es;q=0.9,en;q=0.8")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();

                if status.as_u16() == 429 || status.is_server_error() {
                    limiter.on_throttle().await;
                    let retry_after = parse_retry_after(&resp);
                    backoff(attempt, retry_after).await;
                    continue;
                }

                if status.as_u16() == 401 || status.as_u16() == 403 {
                    limiter.on_success().await;
                    backoff(attempt, None).await;
                    if attempt == max_retries {
                        return FetchOutcome::Transient(format!("autenticación HTTP {status}"));
                    }
                    continue;
                }

                if status.is_client_error() {
                    limiter.on_success().await;
                    return FetchOutcome::NotFound;
                }

                let body = match resp.text().await {
                    Ok(body) => body,
                    Err(err) => {
                        backoff(attempt, None).await;
                        if attempt == max_retries {
                            return FetchOutcome::Transient(format!(
                                "lectura de cuerpo falló: {err}"
                            ));
                        }
                        continue;
                    }
                };

                match classify_body(&body) {
                    BodyKind::Data(student) => {
                        limiter.on_success().await;
                        return FetchOutcome::Found(student);
                    }
                    BodyKind::NotFound => {
                        not_found_seen += 1;
                        if not_found_seen >= confirmations {
                            limiter.on_success().await;
                            return FetchOutcome::NotFound;
                        }
                        sleep(Duration::from_millis(250)).await;
                    }
                    BodyKind::Unknown => {
                        backoff(attempt, None).await;
                    }
                }
            }
            Err(err) => {
                if err.is_timeout() || err.is_connect() {
                    limiter.on_throttle().await;
                }
                backoff(attempt, None).await;
                if attempt == max_retries {
                    return FetchOutcome::Transient(format!("error de red: {err}"));
                }
            }
        }
    }

    FetchOutcome::Transient("reintentos agotados".to_string())
}

fn parse_retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let value = resp.headers().get(RETRY_AFTER)?.to_str().ok()?;
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

async fn backoff(attempt: u32, retry_after: Option<Duration>) {
    if let Some(wait) = retry_after {
        sleep(wait.min(Duration::from_secs(30))).await;
        return;
    }
    let base_ms = 300u64.saturating_mul(1u64 << attempt.min(5));
    let capped = base_ms.min(10_000);
    sleep(Duration::from_millis(capped + jitter_ms())).await;
}

fn jitter_ms() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0);
    let mut x = SEED.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    x % 250
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_not_found_page() {
        let body = r#"<div><b>Estimado usuario, no se ha encontrado ningún estudiante con el número de carnet introducido.</b></div>"#;
        assert!(matches!(classify_body(body), BodyKind::NotFound));
    }

    #[test]
    fn classifies_student_page() {
        let body = r#"
        <div class='row mt-2'><strong>Nombres:</strong></div>
        <div class='row mt-2'>ARI ALEJANDRO</div>
        <div class='row mt-2'><strong>Carnet:</strong></div>
        <div class='row mt-2'>18-IME-0053</div>
        "#;
        match classify_body(body) {
            BodyKind::Data(student) => {
                assert_eq!(student.full_name.as_deref(), Some("ARI ALEJANDRO"))
            }
            other => panic!("esperaba datos, obtuve {other:?}"),
        }
    }

    #[test]
    fn classifies_private_modal_as_unknown() {
        let body = r#"<div class="modal-body"><p>Este contenido es privado.</p></div>"#;
        assert!(matches!(classify_body(body), BodyKind::Unknown));
    }
}
