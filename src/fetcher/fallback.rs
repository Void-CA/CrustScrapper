use futures::stream::{self, StreamExt};
use reqwest::Client;

use crate::domain::carnet::Carnet;
use crate::fetcher::client::{FetchOutcome, fetch_student};
use crate::ratelimit::RateLimiter;
use crate::student::Student;

const FALLBACK_CONCURRENCY: usize = 3;

pub fn changed_candidates(base: &Carnet, careers: &[u8]) -> Vec<Carnet> {
    if base.is_legacy() || base.is_changed() {
        return Vec::new();
    }
    let (year, career, sex, id) = match *base {
        Carnet::New {
            year,
            career,
            sex,
            id,
        } => (year, career, sex, id),
        _ => return Vec::new(),
    };
    if year <= 21 {
        return Vec::new();
    }
    careers
        .iter()
        .copied()
        .filter(|&new_career| new_career != career)
        .map(|new_career| Carnet::Changed {
            year,
            career,
            sex,
            id,
            new_career,
        })
        .collect()
}

pub async fn try_alternate_careers(
    client: &Client,
    limiter: &RateLimiter,
    url: &str,
    base: &Carnet,
    careers: &[u8],
    max_retries: u32,
    not_found_confirmations: u32,
) -> Option<(Carnet, Student)> {
    let candidates = changed_candidates(base, careers);
    if candidates.is_empty() {
        return None;
    }

    let mut stream = stream::iter(candidates.into_iter().map(|carnet| {
        let code = carnet.to_string();
        async move {
            let outcome = fetch_student(
                client,
                limiter,
                url,
                &code,
                max_retries,
                not_found_confirmations,
            )
            .await;
            (carnet, outcome)
        }
    }))
    .buffer_unordered(FALLBACK_CONCURRENCY);

    while let Some((carnet, outcome)) = stream.next().await {
        if let FetchOutcome::Found(student) = outcome {
            return Some((carnet, student));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::carnet::Sex;

    fn base(year: u8) -> Carnet {
        Carnet::New {
            year,
            career: 3,
            sex: Sex::Male,
            id: 41,
        }
    }

    #[test]
    fn only_post_2021_new_carnets_have_candidates() {
        assert!(changed_candidates(&base(21), &[1, 3, 4]).is_empty());
        assert!(
            changed_candidates(
                &Carnet::Legacy {
                    year: 18,
                    acronym: "IME".into(),
                    id: 1
                },
                &[1]
            )
            .is_empty()
        );
        assert!(changed_candidates(&base(22), &[1, 3, 4]).len() == 2);
    }

    #[test]
    fn candidates_exclude_original_career_and_use_real_numbers() {
        let candidates = changed_candidates(&base(22), &[1, 3, 4, 7]);
        let codes: Vec<String> = candidates.iter().map(|c| c.to_string()).collect();
        assert_eq!(
            codes,
            vec![
                "22-A0301-0041-A01",
                "22-A0301-0041-A04",
                "22-A0301-0041-A07"
            ]
        );
    }
}
