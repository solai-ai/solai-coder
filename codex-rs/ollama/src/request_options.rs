use codex_core::config::Config;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use serde::Serialize;
use serde_json::Value;

/// Native request options supported by Ollama's `/api/chat` and `/api/generate`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OllamaRequestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<i64>,
}

impl OllamaRequestOptions {
    fn is_empty(&self) -> bool {
        self.num_ctx.is_none()
    }
}

fn skip_empty_options(options: &Option<OllamaRequestOptions>) -> bool {
    options.as_ref().is_none_or(OllamaRequestOptions::is_empty)
}

/// Native Ollama request body for `/api/generate`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OllamaGenerateRequest<'a> {
    pub model: &'a str,
    pub prompt: &'a str,
    #[serde(skip_serializing_if = "skip_empty_options")]
    pub options: Option<OllamaRequestOptions>,
}

/// Native Ollama request body for `/api/chat`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OllamaChatRequest<'a> {
    pub model: &'a str,
    pub messages: &'a [Value],
    #[serde(skip_serializing_if = "skip_empty_options")]
    pub options: Option<OllamaRequestOptions>,
}

/// Return native Ollama request options when the selected provider is Ollama.
pub fn request_options_for_provider(
    provider_id: &str,
    num_ctx: Option<i64>,
) -> Option<OllamaRequestOptions> {
    if provider_id != OLLAMA_OSS_PROVIDER_ID {
        return None;
    }

    Some(OllamaRequestOptions { num_ctx })
}

/// Return native Ollama request options from a resolved config.
pub fn request_options_for_config(config: &Config) -> Option<OllamaRequestOptions> {
    request_options_for_provider(&config.model_provider_id, config.ollama_num_ctx)
}

/// Build a native `/api/generate` request body.
pub fn build_generate_request_body(
    model: &str,
    prompt: &str,
    options: Option<OllamaRequestOptions>,
) -> Value {
    serde_json::to_value(OllamaGenerateRequest {
        model,
        prompt,
        options,
    })
    .expect("serialize Ollama generate request")
}

/// Build a native `/api/chat` request body.
pub fn build_chat_request_body(
    model: &str,
    messages: &[Value],
    options: Option<OllamaRequestOptions>,
) -> Value {
    serde_json::to_value(OllamaChatRequest {
        model,
        messages,
        options,
    })
    .expect("serialize Ollama chat request")
}
