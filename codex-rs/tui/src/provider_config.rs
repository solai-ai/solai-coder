use serde::Deserialize;
use std::time::Duration;
use url::Url;

const PROVIDER_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedProviderConfig {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) models: Vec<DetectedProviderModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedProviderModel {
    pub(crate) id: String,
    pub(crate) supports_tools: Option<bool>,
    pub(crate) context_window: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProviderConfigError {
    #[error("Enter a provider address like 127.0.0.1:11434 or http://127.0.0.1:1234")]
    EmptyAddress,
    #[error("Invalid provider address: {0}")]
    InvalidAddress(String),
    #[error("Provider did not return any models from /v1/models or /api/tags")]
    NoModels,
    #[error("Could not connect to provider at {base_url}: {message}")]
    ProbeFailed { base_url: String, message: String },
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    details: Option<OllamaModelDetails>,
    capabilities: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct OllamaModelDetails {
    context_length: Option<i64>,
}

pub(crate) async fn detect_provider_config(
    address: &str,
) -> Result<DetectedProviderConfig, ProviderConfigError> {
    let normalized = normalize_provider_address(address)?;
    let client = reqwest::Client::builder()
        .timeout(PROVIDER_PROBE_TIMEOUT)
        .build()
        .map_err(|err| ProviderConfigError::ProbeFailed {
            base_url: normalized.base_url.clone(),
            message: err.to_string(),
        })?;

    let mut probe_errors = Vec::new();
    match fetch_ollama_tags(&client, &normalized.root_url).await {
        Ok(models) if !models.is_empty() => return Ok(normalized.into_detected(models)),
        Ok(_) => {}
        Err(err) => probe_errors.push(err),
    }

    match fetch_openai_models(&client, &normalized.base_url).await {
        Ok(models) if !models.is_empty() => Ok(normalized.into_detected(models)),
        Ok(_) => Err(ProviderConfigError::NoModels),
        Err(err) if probe_errors.is_empty() => Err(ProviderConfigError::ProbeFailed {
            base_url: normalized.base_url,
            message: err,
        }),
        Err(err) => Err(ProviderConfigError::ProbeFailed {
            base_url: normalized.base_url,
            message: format!("{}; {err}", probe_errors.join("; ")),
        }),
    }
}

async fn fetch_openai_models(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<DetectedProviderModel>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("/v1/models returned {}", response.status()));
    }
    let body = response
        .json::<OpenAiModelsResponse>()
        .await
        .map_err(|err| err.to_string())?;
    Ok(body
        .data
        .into_iter()
        .map(|model| DetectedProviderModel {
            id: model.id,
            supports_tools: model
                .capabilities
                .as_ref()
                .map(|capabilities| capabilities.iter().any(|capability| capability == "tools")),
            context_window: None,
        })
        .collect())
}

async fn fetch_ollama_tags(
    client: &reqwest::Client,
    root_url: &str,
) -> Result<Vec<DetectedProviderModel>, String> {
    let url = format!("{}/api/tags", root_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("/api/tags returned {}", response.status()));
    }
    let body = response
        .json::<OllamaTagsResponse>()
        .await
        .map_err(|err| err.to_string())?;
    Ok(body
        .models
        .into_iter()
        .map(|model| {
            let supports_tools = model
                .capabilities
                .as_ref()
                .map(|capabilities| capabilities.iter().any(|capability| capability == "tools"));
            let id = normalize_ollama_model_name(&model.name);
            DetectedProviderModel {
                id,
                supports_tools,
                context_window: model.details.and_then(|details| details.context_length),
            }
        })
        .collect())
}

fn normalize_ollama_model_name(model_name: &str) -> String {
    model_name
        .strip_suffix(":latest")
        .unwrap_or(model_name)
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedProviderAddress {
    provider_id: String,
    provider_name: String,
    base_url: String,
    root_url: String,
}

impl NormalizedProviderAddress {
    fn into_detected(self, models: Vec<DetectedProviderModel>) -> DetectedProviderConfig {
        let model = models
            .iter()
            .find(|model| model.supports_tools == Some(true))
            .unwrap_or(&models[0])
            .id
            .clone();
        DetectedProviderConfig {
            provider_id: self.provider_id,
            provider_name: self.provider_name,
            base_url: self.base_url,
            model,
            models,
        }
    }
}

fn normalize_provider_address(
    address: &str,
) -> Result<NormalizedProviderAddress, ProviderConfigError> {
    let address = address.trim();
    if address.is_empty() {
        return Err(ProviderConfigError::EmptyAddress);
    }

    let with_scheme = if address.contains("://") {
        address.to_string()
    } else {
        format!("http://{address}")
    };
    let mut url = Url::parse(&with_scheme)
        .map_err(|err| ProviderConfigError::InvalidAddress(err.to_string()))?;
    let host = url
        .host_str()
        .ok_or_else(|| ProviderConfigError::InvalidAddress("missing host".to_string()))?
        .to_string();
    let port = url.port_or_known_default();
    if url.path() == "/" {
        url.set_path("/v1");
    }

    let base_url = url.to_string().trim_end_matches('/').to_string();
    let mut root = url;
    root.set_path("");
    root.set_query(None);
    root.set_fragment(None);
    let root_url = root.to_string().trim_end_matches('/').to_string();
    let provider_id = provider_id_for(&host, port);
    let provider_name = provider_name_for(&host, port);

    Ok(NormalizedProviderAddress {
        provider_id,
        provider_name,
        base_url,
        root_url,
    })
}

fn provider_id_for(host: &str, port: Option<u16>) -> String {
    let mut id = String::from("local");
    for ch in host.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else {
            id.push('-');
        }
    }
    if let Some(port) = port {
        id.push('-');
        id.push_str(&port.to_string());
    }
    id
}

fn provider_name_for(host: &str, port: Option<u16>) -> String {
    match port {
        Some(11434) => "Ollama".to_string(),
        Some(1234) => "LM Studio".to_string(),
        Some(port) => format!("Local provider ({host}:{port})"),
        None => format!("Local provider ({host})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[test]
    fn normalizes_bare_host_port_to_v1_base_url() {
        let normalized = normalize_provider_address("127.0.0.1:11434").expect("normalize");

        assert_eq!(
            normalized,
            NormalizedProviderAddress {
                provider_id: "local127-0-0-1-11434".to_string(),
                provider_name: "Ollama".to_string(),
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                root_url: "http://127.0.0.1:11434".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn detects_openai_compatible_models() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id": "openai-compatible-model"}]
            })))
            .mount(&server)
            .await;

        let detected = detect_provider_config(&server.uri())
            .await
            .expect("detected");

        assert_eq!(detected.model, "openai-compatible-model");
        assert_eq!(
            detected.models,
            vec![DetectedProviderModel {
                id: "openai-compatible-model".to_string(),
                supports_tools: None,
                context_window: None,
            }]
        );
        assert_eq!(detected.base_url, format!("{}/v1", server.uri()));
    }

    #[tokio::test]
    async fn detects_openai_compatible_model_tool_capabilities() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": "openai-compatible-tool-model",
                    "capabilities": ["completion", "tools"]
                }]
            })))
            .mount(&server)
            .await;

        let detected = detect_provider_config(&server.uri())
            .await
            .expect("detected");

        assert_eq!(
            detected.models,
            vec![DetectedProviderModel {
                id: "openai-compatible-tool-model".to_string(),
                supports_tools: Some(true),
                context_window: None,
            }]
        );
        assert_eq!(detected.model, "openai-compatible-tool-model");
    }

    #[tokio::test]
    async fn falls_back_to_ollama_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": [{
                    "name": "llama3.1:8b",
                    "details": {"context_length": 131072},
                    "capabilities": ["completion", "tools"]
                }]
            })))
            .mount(&server)
            .await;

        let detected = detect_provider_config(&server.uri())
            .await
            .expect("detected");

        assert_eq!(detected.model, "llama3.1:8b");
        assert_eq!(
            detected.models,
            vec![DetectedProviderModel {
                id: "llama3.1:8b".to_string(),
                supports_tools: Some(true),
                context_window: Some(131072),
            }]
        );
    }

    #[tokio::test]
    async fn ollama_tags_strip_latest_alias() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": [{
                    "name": "custom-local-model:latest",
                    "details": {"context_length": 32768},
                    "capabilities": ["completion", "tools"]
                }]
            })))
            .mount(&server)
            .await;

        let detected = detect_provider_config(&server.uri())
            .await
            .expect("detected");

        assert_eq!(detected.model, "custom-local-model");
        assert_eq!(
            detected.models,
            vec![DetectedProviderModel {
                id: "custom-local-model".to_string(),
                supports_tools: Some(true),
                context_window: Some(32768),
            }]
        );
    }
}
