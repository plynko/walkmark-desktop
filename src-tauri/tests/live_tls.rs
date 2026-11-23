//! Optional check that HTTPS to the real provider APIs works with the TLS
//! stack the app is built with (certificate chains and root certificates
//! included). The stub-server tests in `wire.rs` use plain HTTP and cannot
//! catch a broken TLS setup.
//!
//! No real key is used: a rejected fake key (or a public endpoint that does
//! not ask for one) still proves the encrypted connection was established,
//! which is all this checks.
//!
//! Run with:
//!   RUN_LIVE_TESTS=1 cargo test --manifest-path src-tauri/Cargo.toml --test live_tls -- --nocapture

use walkmark_lib::probe::{provider, Provider};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(future)
}

#[test]
fn every_cloud_provider_answers_over_https() {
    if std::env::var("RUN_LIVE_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipped: set RUN_LIVE_TESTS=1 to reach the real provider APIs");
        return;
    }

    let providers = [
        (Provider::Gemini, "gemini-3.6-flash"),
        (Provider::OpenAi, "gpt-5"),
        (Provider::Anthropic, "claude-sonnet-5-5"),
        (Provider::Mistral, "mistral-small-latest"),
        (Provider::OpenRouter, "openrouter/auto"),
    ];

    for (which, model) in providers {
        let client = provider::Client::new(
            which,
            model.into(),
            which.default_base_url().into(),
            "not-a-real-key".into(),
        )
        .expect("a client");

        // Some providers list their models without a key, so "accepted" is fine
        // too. Only a failure to connect or to verify the certificate is not.
        let message = match block_on(client.check()) {
            Ok(()) => "answered (the key was not needed)".to_string(),
            Err(e) => e.to_string(),
        };
        println!("{}: {message}", which.label());
        assert!(
            !message.starts_with("Network request failed"),
            "{} could not be reached over HTTPS: {message}",
            which.label()
        );
    }
}
