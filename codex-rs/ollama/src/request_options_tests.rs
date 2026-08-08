use super::OllamaRequestOptions;
use super::build_chat_request_body;
use super::build_generate_request_body;
use super::request_options_for_provider;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn generate_request_includes_num_ctx_when_configured() {
    let body = build_generate_request_body(
        "qwen30",
        "responda apenas: ok",
        Some(OllamaRequestOptions {
            num_ctx: Some(32_768),
        }),
    );

    assert_eq!(
        body,
        json!({
            "model": "qwen30",
            "prompt": "responda apenas: ok",
            "options": {
                "num_ctx": 32_768
            }
        })
    );
}

#[test]
fn chat_request_includes_num_ctx_when_configured() {
    let body = build_chat_request_body(
        "qwen30",
        &[json!({"role": "user", "content": "responda apenas: ok"})],
        Some(OllamaRequestOptions {
            num_ctx: Some(32_768),
        }),
    );

    assert_eq!(
        body,
        json!({
            "model": "qwen30",
            "messages": [
                {"role": "user", "content": "responda apenas: ok"}
            ],
            "options": {
                "num_ctx": 32_768
            }
        })
    );
}

#[test]
fn request_options_omits_num_ctx_when_unset() {
    let body = build_generate_request_body(
        "qwen30",
        "responda apenas: ok",
        Some(OllamaRequestOptions::default()),
    );

    assert_eq!(
        body,
        json!({
            "model": "qwen30",
            "prompt": "responda apenas: ok"
        })
    );
}

#[test]
fn request_options_return_none_for_other_providers() {
    assert_eq!(request_options_for_provider("llamacpp", Some(32_768)), None);
}

#[test]
fn request_options_return_num_ctx_only_for_ollama() {
    assert_eq!(
        request_options_for_provider("ollama", Some(32_768)),
        Some(OllamaRequestOptions {
            num_ctx: Some(32_768)
        })
    );
}
