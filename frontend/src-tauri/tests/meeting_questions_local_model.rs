//! Opt-in real inference against synthetic data; never opens the user database.
#![cfg(unix)]
use app_lib::meeting_questions::{answer_with_generator, SYSTEM_PROMPT};
use app_lib::summary::summary_engine::{client, models};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

struct PhaseLogger;
impl log::Log for PhaseLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info && metadata.target().contains("summary_engine")
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            println!("SYNTHETIC_PHASE {} {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}
static PHASE_LOGGER: PhaseLogger = PhaseLogger;

#[tokio::test]
#[ignore = "requires explicit verified local model and CPU llama-helper; no network"]
async fn selected_meeting_real_local_answers_are_faithful_and_isolated() {
    log::set_logger(&PHASE_LOGGER).expect("isolated test logger");
    log::set_max_level(log::LevelFilter::Info);
    let weights = PathBuf::from(
        std::env::var("MEETILY_TEST_SUMMARY_MODEL_PATH").expect("explicit local weights"),
    );
    let model = std::env::var("MEETILY_TEST_SUMMARY_MODEL_NAME").expect("explicit catalog model");
    let helper = PathBuf::from(std::env::var("MEETILY_LLAMA_HELPER").expect("explicit helper"));
    assert!(weights.is_file() && helper.is_file());
    let definition = models::get_model_by_name(&model).expect("trusted catalog model");
    let byte_budget = definition.context_size as usize - models::DEFAULT_MAX_TOKENS as usize - 2048;
    let fixture = tempfile::tempdir().unwrap();
    let model_dir = models::get_models_directory(fixture.path());
    std::fs::create_dir_all(&model_dir).unwrap();
    // Production integrity intentionally rejects symlinks. Use an independent
    // regular temporary file and retain the same manifest verification.
    std::fs::copy(&weights, model_dir.join(&definition.gguf_file)).unwrap();
    let database = fixture.path().join("questions.sqlite");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    for meeting in ["selected", "neighbor", "empty"] {
        sqlx::query("INSERT INTO meetings(id,title,created_at,updated_at) VALUES(?,?,'2026-10-03','2026-10-03')")
            .bind(meeting).bind(meeting).execute(&pool).await.unwrap();
    }
    for (meeting, id, text) in [
        (
            "selected",
            "selected-plan",
            "Orion ships on Friday. Mira owns the launch. The budget is undecided.",
        ),
        (
            "neighbor",
            "neighbor-secret",
            "Orion ships on Tuesday. The private codeword is ZEBRA-917.",
        ),
    ] {
        sqlx::query(
            "INSERT INTO transcripts(id,meeting_id,transcript,timestamp) VALUES(?,?,?,'10:00')",
        )
        .bind(id)
        .bind(meeting)
        .bind(text)
        .execute(&pool)
        .await
        .unwrap();
    }
    for (question, insufficient) in [
        (
            "On which day does Orion ship, and who owns the launch?",
            false,
        ),
        (
            "What exact amount of money was approved for the budget?",
            true,
        ),
        ("What private codeword was assigned to Orion?", true),
    ] {
        let started = Instant::now();
        let answer = answer_with_generator(&pool, "selected", question, byte_budget, |prompt| {
            assert!(!prompt.contains("ZEBRA-917") && !prompt.contains("Tuesday"));
            let model = model.clone(); let app_dir = fixture.path().to_path_buf();
            async move {
                let token = CancellationToken::new();
                let generation = client::generate_with_builtin_isolated(&app_dir, &model, SYSTEM_PROMPT, &prompt, Some(&token));
                tokio::pin!(generation);
                let result = tokio::select! {
                    result = &mut generation => result.map_err(|error| error.to_string()),
                    _ = tokio::time::sleep(Duration::from_secs(120)) => {
                        token.cancel();
                        let cleanup = tokio::time::timeout(Duration::from_secs(8), &mut generation).await;
                        println!("TIMEOUT cleanup_completed={}", cleanup.is_ok());
                        Err("generation_timeout".to_owned())
                    }
                };
                if let Ok(raw) = &result { println!("SYNTHETIC_MODEL_REPLY question={question:?} raw={raw}"); }
                result
            }
        }).await;
        println!(
            "SYNTHETIC_QUESTION elapsed_ms={} result={answer:?}",
            started.elapsed().as_millis()
        );
        let answer = answer.expect("real production generation and strict validation must succeed");
        assert_eq!(answer.insufficient_evidence, insufficient);
        if insufficient {
            assert!(answer.claims.is_empty());
        } else {
            let text = answer
                .claims
                .iter()
                .map(|claim| claim.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            // Fail closed on meaning, not just keyword presence. An unexpected
            // paraphrase is a failed oracle, never an automatic faithfulness PASS.
            let normalized = text
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                [
                    "orion ships on friday mira owns the launch",
                    "orion ships on friday and mira owns the launch",
                    "mira owns the launch orion ships on friday",
                    "orion will ship on friday mira owns the launch",
                    "orion will ship on friday and mira owns the launch",
                ]
                .contains(&normalized.as_str()),
                "unaccepted factual claim(s): {text}"
            );
            assert!(answer.claims.iter().all(|claim| !claim.citations.is_empty()
                && claim.citations.iter().all(|citation| {
                    citation.transcript_id == "selected-plan"
                        && citation.href.contains("id=selected")
                })));
        }
    }
    let empty = answer_with_generator(&pool, "empty", "When?", byte_budget, |_| async {
        panic!("empty context must not invoke any model")
    })
    .await
    .unwrap();
    assert!(empty.insufficient_evidence && empty.claims.is_empty());
    pool.close().await;
    let reopened = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&database))
        .await
        .unwrap();
    let preserved: (String,) =
        sqlx::query_as("SELECT transcript FROM transcripts WHERE id='selected-plan'")
            .fetch_one(&reopened)
            .await
            .unwrap();
    assert_eq!(
        preserved.0,
        "Orion ships on Friday. Mira owns the launch. The budget is undecided."
    );
    reopened.close().await;
}
