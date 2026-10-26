use super::*;

#[tokio::test]
async fn local_transport_bypasses_system_proxy_while_network_transport_preserves_it() {
    const CHILD_ENDPOINT: &str = "MEETILY_TEST_QUESTION_PROXY_ENDPOINT";
    if let Ok(endpoint) = std::env::var(CHILD_ENDPOINT) {
        for (mode, expected) in [("local", "origin"), ("network", "proxy")] {
            let response = question_http_client(mode)
                .unwrap()
                .post(&endpoint)
                .body("synthetic selected meeting")
                .send()
                .await
                .unwrap();
            assert_eq!(response.text().await.unwrap(), expected);
        }
        return;
    }

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/question", origin.local_addr().unwrap());
    let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
    let serve = |listener: tokio::net::TcpListener, body: &'static str| async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"synthetic selected meeting") {
            let mut bytes = [0; 1024];
            let size = socket.read(&mut bytes).await.unwrap();
            assert!(size > 0 && request.len() < 4096);
            request.extend_from_slice(&bytes[..size]);
        }
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    };
    let origin_server = tokio::spawn(serve(origin, "origin"));
    let proxy_server = tokio::spawn(serve(proxy, "proxy"));
    // Isolate process-global proxy environment and reqwest's proxy cache.
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "meeting_questions::request_tests::local_transport_bypasses_system_proxy_while_network_transport_preserves_it",
            "--nocapture",
        ])
        .env(CHILD_ENDPOINT, endpoint)
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .env_remove("REQUEST_METHOD")
        .kill_on_drop(true);
    for name in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        child.env(name, &proxy_url);
    }
    let output = tokio::time::timeout(Duration::from_secs(15), child.output())
        .await
        .expect("proxy regression subprocess timed out")
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        origin_server.await.unwrap();
        proxy_server.await.unwrap();
    })
    .await
    .expect("both origin and proxy must receive their respective requests");
}

#[test]
fn endpoints_are_classified_by_actual_loopback_host_without_redirects() {
    for host in [
        "http://localhost:11434",
        "http://127.0.0.2:8000/v1",
        "http://[::1]:8080",
    ] {
        assert_eq!(
            transport(&LLMProvider::Ollama, Some(host)).unwrap(),
            "local"
        );
    }
    for host in [
        "https://api.example.test/v1",
        "http://192.168.1.2:8000",
        "http://localhost.evil.test:8000",
    ] {
        assert_eq!(
            transport(&LLMProvider::CustomOpenAI, Some(host)).unwrap(),
            "network"
        );
    }
    assert!(transport(&LLMProvider::CustomOpenAI, Some("file:///tmp/key")).is_err());
    assert!(transport(
        &LLMProvider::CustomOpenAI,
        Some("https://secret:password@example.test")
    )
    .is_err());
}
#[tokio::test]
async fn config_fingerprint_binds_endpoint_and_generation_parameters() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    SettingsRepository::save_model_config(
        &pool,
        "ollama",
        "fixture-model",
        "fixture-transcriber",
        Some("http://localhost:11434"),
    )
    .await
    .unwrap();
    let first = selection(&pool).await.unwrap();
    assert_eq!(first.visible.transport, "local");
    SettingsRepository::save_model_config(
        &pool,
        "ollama",
        "fixture-model",
        "fixture-transcriber",
        Some("https://remote.example.test"),
    )
    .await
    .unwrap();
    let second = selection(&pool).await.unwrap();
    assert_eq!(second.visible.transport, "network");
    assert_ne!(first.visible.fingerprint, second.visible.fingerprint);
    assert_eq!(first.visible.provider, second.visible.provider);
    assert_eq!(first.visible.model, second.visible.model);
    let exposed = serde_json::to_value(&second.visible).unwrap();
    assert!(exposed.get("apiKey").is_none());
    assert!(exposed.get("endpoint").is_none());
    let custom = CustomOpenAIConfig {
        endpoint: "http://127.0.0.1:8080/v1".into(),
        model: "fixture".into(),
        api_key: None,
        max_tokens: Some(1000),
        temperature: None,
        top_p: None,
    };
    SettingsRepository::save_model_config(
        &pool,
        "custom-openai",
        "fixture",
        "fixture-transcriber",
        None,
    )
    .await
    .unwrap();
    SettingsRepository::save_custom_openai_config(&pool, &custom)
        .await
        .unwrap();
    let a = selection(&pool).await.unwrap();
    let custom = CustomOpenAIConfig {
        max_tokens: Some(2000),
        ..custom
    };
    SettingsRepository::save_custom_openai_config(&pool, &custom)
        .await
        .unwrap();
    assert_ne!(
        a.visible.fingerprint,
        selection(&pool).await.unwrap().visible.fingerprint
    );
}
#[tokio::test]
async fn http_cancel_interrupts_delayed_success_and_error_bodies() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for status in ["200 OK", "500 Internal Server Error"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (headers_sent, received) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer).await.unwrap();
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: 9999\r\n\r\n").as_bytes()).await.unwrap();
            headers_sent.send(()).unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let endpoint = format!("http://{address}/v1");
        let selected = NativeSelection {
            visible: QuestionConfiguration {
                provider: "custom-openai".into(),
                model: "fixture".into(),
                transport: "local".into(),
                fingerprint: "fixture".into(),
            },
            provider: LLMProvider::CustomOpenAI,
            api_key: String::new(),
            endpoint: Some(endpoint.clone()),
            custom: Some(CustomOpenAIConfig {
                endpoint,
                model: "fixture".into(),
                api_key: None,
                max_tokens: Some(100),
                temperature: None,
                top_p: None,
            }),
            byte_budget: 24000,
        };
        let token = CancellationToken::new();
        let request_token = token.clone();
        let mut request = tokio::spawn(async move {
            generate(
                &selected,
                Path::new("/private/tmp"),
                "fixture question",
                &request_token,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(3), received)
            .await
            .unwrap()
            .unwrap();
        // Headers are available and the adapter is waiting for a body, not
        // still waiting to establish the request.
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
        let result = tokio::time::timeout(Duration::from_millis(300), &mut request).await;
        request.abort();
        server.abort();
        let _ = server.await;
        assert_eq!(
            result
                .expect("cancellation did not interrupt body read")
                .unwrap()
                .unwrap_err(),
            "cancelled"
        );
    }
}

#[tokio::test]
async fn request_ids_cancel_before_dispatch_and_do_not_cancel_other_meetings() {
    let id = uuid::Uuid::new_v4().to_string();
    api_cancel_meeting_question("selected".into(), id.clone())
        .await
        .unwrap();
    assert!(
        matches!(RequestRegistration::acquire(&id,"selected"),Err(error) if error=="cancelled")
    );
    let id = uuid::Uuid::new_v4().to_string();
    let request = RequestRegistration::acquire(&id, "selected").unwrap();
    assert!(api_cancel_meeting_question("neighbor".into(), id.clone())
        .await
        .is_err());
    assert!(!request.token.is_cancelled());
    assert!(RequestRegistration::acquire(&id, "selected").is_err());
    api_cancel_meeting_question("selected".into(), id.clone())
        .await
        .unwrap();
    assert!(request.token.is_cancelled());
    drop(request);
    assert!(!REQUESTS.lock().unwrap().contains_key(&id));
}
