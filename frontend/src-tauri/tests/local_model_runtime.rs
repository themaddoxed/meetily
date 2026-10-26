use anyhow::{anyhow, bail, Context, Result};
use app_lib::audio::decode_audio_file;
use app_lib::parakeet_engine::{ModelStatus, ParakeetEngine};
use app_lib::summary::summary_engine::models::{format_prompt, get_model_by_name};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

const IO_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);
const PARAKEET_TIMEOUT: Duration = Duration::from_secs(180);
const GENERATION_TIMEOUT: Duration = Duration::from_secs(180);
const SUMMARY_HELPER_CONTEXT_CAP: u32 = 1024;
const SUMMARY_HELPER_MAX_TOKENS: i32 = 48;
const CONTROL_MARKERS: &[&str] = &[
    "<unused",
    "[multimodal]",
    "<start_of_turn>",
    "<end_of_turn>",
    "<|turn>",
    "<turn|>",
    "<|im_start|>",
    "<|im_end|>",
    "<think>",
    "</think>",
    "<|tool_call>",
    "<tool_call|>",
    "<|tool_response>",
    "<tool_response|>",
    "<|channel>",
    "<channel|>",
];

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum HelperResponse {
    Pong,
    Goodbye,
    #[serde(rename = "response")]
    Generated {
        text: String,
        error: Option<String>,
    },
    Error {
        message: String,
    },
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name)
        .with_context(|| format!("{name} must be set explicitly for this ignored smoke test"))?;
    if value.trim().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

fn required_path(name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(required_env(name)?);
    if !path.exists() {
        bail!("{name} does not exist: {}", path.display());
    }
    Ok(path)
}

fn normalized_words(value: &str) -> Vec<String> {
    let normalized: String = value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect();
    normalized
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect()
}

fn word_error_rate(actual: &str, expected: &str) -> Result<f32> {
    let expected_words = normalized_words(expected);
    if expected_words.is_empty() {
        bail!("expected text must contain at least one letter or number");
    }
    let actual_words = normalized_words(actual);

    let mut previous: Vec<usize> = (0..=actual_words.len()).collect();
    for (expected_index, expected_word) in expected_words.iter().enumerate() {
        let mut current = Vec::with_capacity(actual_words.len() + 1);
        current.push(expected_index + 1);

        for (actual_index, actual_word) in actual_words.iter().enumerate() {
            let insertion = current[actual_index] + 1;
            let deletion = previous[actual_index + 1] + 1;
            let substitution = previous[actual_index] + usize::from(expected_word != actual_word);
            current.push(insertion.min(deletion).min(substitution));
        }

        previous = current;
    }

    Ok(previous[actual_words.len()] as f32 / expected_words.len() as f32)
}

fn validate_parakeet_transcript(actual: &str, expected: &str) -> Result<()> {
    let error_rate = word_error_rate(actual, expected)?;
    if error_rate > 0.4 {
        bail!(
            "Parakeet transcript word error rate exceeded 40% ({error_rate:.0}%): {actual}",
            error_rate = error_rate * 100.0
        );
    }

    Ok(())
}

fn build_summary_generate_request(
    model_name: &str,
    model_path: &std::path::Path,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<Value> {
    let model_def = get_model_by_name(model_name)
        .ok_or_else(|| anyhow!("unknown registered summary model: {model_name}"))?;
    let actual_filename = model_path
        .file_name()
        .and_then(|filename| filename.to_str())
        .ok_or_else(|| {
            anyhow!(
                "summary model path has no UTF-8 filename: {}",
                model_path.display()
            )
        })?;
    if actual_filename != model_def.gguf_file {
        bail!(
            "summary model file `{actual_filename}` does not match registered model `{model_name}` file `{}`",
            model_def.gguf_file
        );
    }

    let prompt = format_prompt(&model_def.template, system_prompt, user_prompt)?;
    let sampling = model_def.sampling.sanitize_for_llama_helper();

    Ok(json!({
        "type": "generate",
        "prompt": prompt,
        "max_tokens": SUMMARY_HELPER_MAX_TOKENS,
        "context_size": model_def.context_size.min(SUMMARY_HELPER_CONTEXT_CAP),
        "model_path": model_path,
        "temperature": sampling.temperature,
        "top_k": sampling.top_k,
        "top_p": sampling.top_p,
        "presence_penalty": sampling.presence_penalty,
        "frequency_penalty": sampling.frequency_penalty,
        "repeat_penalty": sampling.repeat_penalty,
        "penalty_last_n": sampling.penalty_last_n,
        "stop_tokens": sampling.stop_tokens,
    }))
}

fn validate_generated_response(response: HelperResponse, expected: &str) -> Result<String> {
    let HelperResponse::Generated { text, error } = response else {
        bail!("llama-helper did not return a generation response");
    };

    if let Some(error) = error {
        bail!("llama-helper generation failed: {error}");
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        bail!("llama-helper returned an empty response");
    }

    let lower = trimmed.to_lowercase();
    if CONTROL_MARKERS.iter().any(|marker| lower.contains(marker)) {
        bail!("llama-helper leaked a model control marker: {trimmed}");
    }

    let actual_words = normalized_words(trimmed);
    let expected_words = normalized_words(expected);
    if expected_words.is_empty() {
        bail!("expected text must contain at least one letter or number");
    }
    if actual_words != expected_words {
        bail!(
            "llama-helper did not return the exact normalized reply; expected `{}`, got `{}`",
            expected_words.join(" "),
            actual_words.join(" ")
        );
    }

    Ok(trimmed.to_string())
}

async fn write_request(stdin: &mut ChildStdin, request: &Value) -> Result<()> {
    let mut encoded = serde_json::to_vec(request)?;
    encoded.push(b'\n');

    timeout(IO_TIMEOUT, async {
        stdin.write_all(&encoded).await?;
        stdin.flush().await
    })
    .await
    .context("timed out writing to llama-helper")??;

    Ok(())
}

async fn read_response(
    stdout: &mut Lines<BufReader<ChildStdout>>,
    duration: Duration,
    stage: &str,
) -> Result<HelperResponse> {
    let line = timeout(duration, stdout.next_line())
        .await
        .with_context(|| format!("llama-helper timed out during {stage}"))??
        .ok_or_else(|| anyhow!("llama-helper closed stdout during {stage}"))?;

    serde_json::from_str(&line)
        .with_context(|| format!("invalid llama-helper response during {stage}: {line}"))
}

async fn shutdown_helper(
    child: &mut Child,
    stdin: &mut ChildStdin,
    stdout: &mut Lines<BufReader<ChildStdout>>,
) -> Result<()> {
    let graceful = async {
        write_request(stdin, &json!({ "type": "shutdown" })).await?;
        match read_response(stdout, IO_TIMEOUT, "shutdown").await? {
            HelperResponse::Goodbye => Ok(()),
            other => bail!("unexpected llama-helper shutdown response: {other:?}"),
        }
    }
    .await;

    if graceful.is_err() {
        let _ = child.start_kill();
    }

    timeout(IO_TIMEOUT, child.wait())
        .await
        .context("llama-helper did not exit after shutdown")??;
    graceful
}

#[tokio::test]
#[ignore = "requires explicit local Parakeet model and speech fixture"]
async fn parakeet_installed_model_transcribes_known_speech() -> Result<()> {
    let models_dir = required_path("MEETILY_TEST_MODELS_DIR")?;
    let audio_path = required_path("MEETILY_TEST_AUDIO_PATH")?;
    let expected_text = required_env("MEETILY_TEST_EXPECTED_TEXT")?;
    let model_name = std::env::var("MEETILY_TEST_PARAKEET_MODEL")
        .unwrap_or_else(|_| "parakeet-tdt-0.6b-v3-int8".to_string());

    let model_dir = models_dir.join("parakeet").join(&model_name);
    if !model_dir.is_dir() {
        bail!(
            "explicit Parakeet model directory does not exist: {}",
            model_dir.display()
        );
    }

    let decoded = decode_audio_file(&audio_path)?;
    if decoded.samples.is_empty() {
        bail!("speech fixture decoded to zero samples");
    }
    let peak = decoded
        .samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    let rms = (decoded
        .samples
        .iter()
        .map(|sample| sample * sample)
        .sum::<f32>()
        / decoded.samples.len() as f32)
        .sqrt();
    if rms <= 0.001 || peak <= 0.01 {
        bail!("speech fixture is silent or too quiet (RMS {rms:.6}, peak {peak:.6})");
    }

    let audio = decoded.to_whisper_format();
    if audio.is_empty() {
        bail!("speech fixture produced no 16 kHz mono samples");
    }

    let engine = ParakeetEngine::new_with_models_dir(Some(models_dir))?;
    let models = engine.discover_models().await?;
    let selected = models
        .iter()
        .find(|model| model.name == model_name)
        .ok_or_else(|| anyhow!("Parakeet model was not discovered: {model_name}"))?;
    if !matches!(selected.status, ModelStatus::Available) {
        bail!(
            "Parakeet model is not available: {} ({:?})",
            selected.path.display(),
            selected.status
        );
    }

    timeout(PARAKEET_TIMEOUT, engine.load_model(&model_name))
        .await
        .context("timed out loading local Parakeet model")??;
    let transcript = timeout(PARAKEET_TIMEOUT, engine.transcribe_audio(audio))
        .await
        .context("timed out transcribing the local speech fixture")??;
    if transcript.trim().is_empty() {
        bail!("Parakeet returned an empty transcript");
    }

    validate_parakeet_transcript(&transcript, &expected_text)?;

    println!("PASS: local Parakeet transcript: {transcript}");
    Ok(())
}

#[tokio::test]
#[ignore = "requires explicit local llama-helper and summary model"]
async fn local_summary_helper_generates_expected_text() -> Result<()> {
    let helper_path = required_path("MEETILY_LLAMA_HELPER")?;
    let model_path = required_path("MEETILY_TEST_SUMMARY_MODEL_PATH")?;
    let model_name = required_env("MEETILY_TEST_SUMMARY_MODEL_NAME")?;
    let expected_text = required_env("MEETILY_TEST_SUMMARY_EXPECTED_TEXT")?;

    if !helper_path.is_file() || !model_path.is_file() {
        bail!("llama-helper and summary model paths must point to files");
    }

    let request = build_summary_generate_request(
        &model_name,
        &model_path,
        "You are an offline release-smoke assistant. Follow the requested response exactly.",
        "The meeting decided that the dark theme launches Friday. Reply exactly: Dark theme Friday.",
    )?;

    let mut child = Command::new(&helper_path)
        .env("LLAMA_IDLE_TIMEOUT", "240")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("failed to launch {}", helper_path.display()))?;
    let mut stdin = child
        .stdin
        .take()
        .context("llama-helper stdin was not piped")?;
    let stdout = child
        .stdout
        .take()
        .context("llama-helper stdout was not piped")?;
    let mut stdout = BufReader::new(stdout).lines();

    let exchange = async {
        write_request(&mut stdin, &json!({ "type": "ping" })).await?;
        match read_response(&mut stdout, STARTUP_TIMEOUT, "startup ping").await? {
            HelperResponse::Pong => {}
            HelperResponse::Error { message } => bail!("llama-helper ping failed: {message}"),
            other => bail!("unexpected llama-helper ping response: {other:?}"),
        }

        write_request(&mut stdin, &request).await?;

        read_response(&mut stdout, GENERATION_TIMEOUT, "model load and generation").await
    }
    .await;

    let shutdown = shutdown_helper(&mut child, &mut stdin, &mut stdout).await;
    let response = exchange?;
    shutdown.context("llama-helper failed to shut down cleanly")?;

    let generated = validate_generated_response(response, &expected_text)?;
    println!(
        "PASS: helper/model compatibility at test-only {SUMMARY_HELPER_CONTEXT_CAP}-token context (not full production summary): {generated}"
    );
    Ok(())
}

#[test]
fn normalization_is_case_and_punctuation_insensitive() {
    let error_rate = word_error_rate("DARK-theme Friday!", "dark theme Friday")
        .expect("normalization should accept words");
    assert_eq!(error_rate, 0.0);
}

#[test]
fn response_validation_rejects_control_marker_leakage() {
    let error = validate_generated_response(
        HelperResponse::Generated {
            text: "Dark theme Friday <unused42>".to_string(),
            error: None,
        },
        "dark Friday",
    )
    .expect_err("control markers must fail the local summary smoke");

    assert!(error.to_string().contains("control marker"));
}

#[test]
fn response_validation_requires_all_expected_summary_words() {
    let error = validate_generated_response(
        HelperResponse::Generated {
            text: "Dark theme launches soon".to_string(),
            error: None,
        },
        "dark Friday",
    )
    .expect_err("missing expected words must fail the local summary smoke");

    assert!(error.to_string().contains("exact normalized reply"));
}

#[test]
fn response_validation_rejects_reordered_expected_words() {
    let error = validate_generated_response(
        HelperResponse::Generated {
            text: "Friday dark theme launches".to_string(),
            error: None,
        },
        "dark theme launches Friday",
    )
    .expect_err("reordered words must fail an exact-reply smoke test");

    assert!(error.to_string().contains("exact normalized reply"));
}

#[test]
fn response_validation_rejects_negated_reply_with_all_expected_words() {
    let error = validate_generated_response(
        HelperResponse::Generated {
            text: "Dark theme launches Friday not".to_string(),
            error: None,
        },
        "dark theme launches Friday",
    )
    .expect_err("a negated reply must fail even when all expected words are present");

    assert!(error.to_string().contains("exact normalized reply"));
}

#[test]
fn response_validation_rejects_production_template_marker_leakage() {
    for marker in [
        "<unused42>",
        "[multimodal]",
        "<start_of_turn>",
        "<end_of_turn>",
        "<|turn>",
        "<turn|>",
        "<|im_start|>",
        "<|im_end|>",
        "<think>",
        "</think>",
        "<|tool_call>",
        "<tool_call|>",
        "<|tool_response>",
        "<tool_response|>",
        "<|channel>",
        "<channel|>",
    ] {
        let error = validate_generated_response(
            HelperResponse::Generated {
                text: format!("Dark theme Friday {marker}"),
                error: None,
            },
            "dark theme Friday",
        )
        .expect_err("production template markers must fail the local summary smoke");

        assert!(
            error.to_string().contains("control marker"),
            "unexpected validation error for {marker}: {error}"
        );
    }
}

#[test]
fn summary_smoke_request_uses_registered_production_model_metadata() {
    let model = app_lib::summary::summary_engine::models::get_model_by_name("qwen3.5:4b")
        .expect("registered model must exist");
    let model_path = PathBuf::from("/tmp").join(&model.gguf_file);
    let request = build_summary_generate_request("qwen3.5:4b", &model_path, "system", "user")
        .expect("registered model metadata should build a request");
    let sampling = model.sampling.sanitize_for_llama_helper();

    assert_eq!(request["context_size"], SUMMARY_HELPER_CONTEXT_CAP);
    assert_eq!(request["max_tokens"], SUMMARY_HELPER_MAX_TOKENS);
    assert_eq!(request["temperature"], sampling.temperature);
    assert_eq!(request["top_k"], sampling.top_k);
    assert_eq!(request["top_p"], sampling.top_p);
    assert_eq!(request["stop_tokens"], json!(sampling.stop_tokens));
}

#[test]
fn summary_smoke_request_rejects_wrong_registered_model_filename() {
    let error = build_summary_generate_request(
        "qwen3.5:4b",
        &PathBuf::from("/tmp/gemma-4-E2B-it-Q4_K_M.gguf"),
        "system",
        "user",
    )
    .expect_err("a model name and GGUF filename mismatch must fail");

    assert!(error
        .to_string()
        .contains("does not match registered model"));
}

#[test]
fn parakeet_metric_rejects_excess_insertions() {
    let error = validate_parakeet_transcript(
        "meetily records local notes with many unrelated extra tokens",
        "meetily records local notes",
    )
    .expect_err("excess inserted words must fail the Parakeet smoke");

    assert!(error.to_string().contains("word error rate"));
}

#[test]
fn parakeet_metric_rejects_reordered_words() {
    let error = validate_parakeet_transcript(
        "friday safely notes meeting local records meetily",
        "meetily records local meeting notes safely friday",
    )
    .expect_err("severely reordered words must fail the Parakeet smoke");

    assert!(error.to_string().contains("word error rate"));
}
