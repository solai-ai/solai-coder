use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::default_input_modalities;
use std::convert::Infallible;

use crate::provider_config::DetectedProviderModel;

#[derive(Debug, Clone)]
pub(crate) struct ModelCatalog {
    models: Vec<ModelPreset>,
}

impl ModelCatalog {
    pub(crate) fn new(models: Vec<ModelPreset>) -> Self {
        Self { models }
    }

    pub(crate) fn try_list_models(&self) -> Result<Vec<ModelPreset>, Infallible> {
        Ok(self.models.clone())
    }
}

pub(crate) fn local_provider_model_presets(models: &[DetectedProviderModel]) -> Vec<ModelPreset> {
    models
        .iter()
        .enumerate()
        .map(|(index, model)| ModelPreset {
            id: model.id.clone(),
            model: model.id.clone(),
            display_name: model.id.clone(),
            description: local_provider_model_description(model),
            default_reasoning_effort: ReasoningEffort::None,
            supported_reasoning_efforts: vec![ReasoningEffortPreset {
                effort: ReasoningEffort::None,
                description: "No reasoning".to_string(),
            }],
            supports_personality: false,
            additional_speed_tiers: Vec::new(),
            service_tiers: Vec::new(),
            default_service_tier: None,
            is_default: index == 0,
            upgrade: None,
            show_in_picker: true,
            availability_nux: None,
            supported_in_api: true,
            input_modalities: default_input_modalities(),
        })
        .collect()
}

fn local_provider_model_description(model: &DetectedProviderModel) -> String {
    let tool_label = match model.supports_tools {
        Some(true) => "tools enabled",
        Some(false) => "tools unavailable",
        None => "tool support unknown",
    };
    match model.context_window {
        Some(context_window) => {
            format!("Local provider model; {tool_label}; {context_window} context tokens")
        }
        None => format!("Local provider model; {tool_label}"),
    }
}
