/// Identify whether a base_url points at an SolaiAgent-compatible root (".../v1").
pub(crate) fn is_openai_compatible_base_url(base_url: &str) -> bool {
    base_url.trim_end_matches('/').ends_with("/v1")
}

/// Convert a provider base_url into the native Ollama host root.
/// For example, "http://localhost:11434/v1" -> "http://localhost:11434".
pub fn base_url_to_host_root(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed
            .trim_end_matches("/v1")
            .trim_end_matches('/')
            .to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn is_local_ollama_base_url(base_url: &str) -> bool {
    let host_root = base_url_to_host_root(base_url);
    let without_scheme = host_root
        .split_once("://")
        .map_or(host_root.as_str(), |(_, rest)| rest);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host = authority
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(host, _)| host))
        .unwrap_or_else(|| authority.split(':').next().unwrap_or(authority));

    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url_to_host_root() {
        assert_eq!(
            base_url_to_host_root("http://localhost:11434/v1"),
            "http://localhost:11434"
        );
        assert_eq!(
            base_url_to_host_root("http://localhost:11434"),
            "http://localhost:11434"
        );
        assert_eq!(
            base_url_to_host_root("http://localhost:11434/"),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_is_local_ollama_base_url() {
        assert!(is_local_ollama_base_url("http://localhost:11434/v1"));
        assert!(is_local_ollama_base_url("http://127.0.0.1:11434/v1"));
        assert!(is_local_ollama_base_url("http://[::1]:11434/v1"));
        assert!(!is_local_ollama_base_url("http://192.168.100.33:11434/v1"));
    }
}
