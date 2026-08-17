use chrono::DateTime;
use chrono::Utc;
use color_eyre::eyre::Result;
use color_eyre::eyre::bail;
use serde::Deserialize;
use std::path::PathBuf;

const MARKETPLACE_LEASES_FILE: &str = "marketplace-leases.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarketplaceLease {
    pub(crate) id: String,
    pub(crate) provider: String,
    pub(crate) endpoint: String,
    pub(crate) model: String,
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) status: Option<String>,
    pub(crate) auth_token: String,
}

#[derive(Debug, Deserialize)]
struct MarketplaceLeasesFile {
    leases: Vec<MarketplaceLease>,
}

#[derive(Debug, Clone)]
pub(crate) struct MarketplaceLeaseStatus {
    pub(crate) id: String,
    pub(crate) provider: String,
    pub(crate) endpoint: String,
    pub(crate) model: String,
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) status: String,
    pub(crate) using_now: bool,
}

impl MarketplaceLease {
    pub(crate) fn provider_key(&self) -> String {
        format!("solai-lease-{}", sanitize_provider_key(&self.id))
    }

    pub(crate) fn provider_name(&self) -> String {
        format!("SOLAI Lease {}", self.id)
    }

    pub(crate) fn base_url(&self) -> String {
        format!("{}/v1", self.endpoint.trim_end_matches('/'))
    }

    pub(crate) fn is_active(&self) -> bool {
        self.status.as_deref().unwrap_or("ACTIVE") == "ACTIVE" && self.expires_at > Utc::now()
    }
}

pub(crate) fn current_marketplace_lease_status(
    model_provider_id: &str,
    model: Option<&str>,
    base_url: Option<&str>,
) -> Option<MarketplaceLeaseStatus> {
    let leases = load_marketplace_leases_sync().ok()?;
    let lease = leases.leases.into_iter().max_by_key(|lease| lease.starts_at)?;
    let lease_base_url = lease.base_url();
    let active = lease.is_active();
    let using_now = active
        && model_provider_id == lease.provider_key()
        && model == Some(lease.model.as_str())
        && base_url == Some(lease_base_url.as_str());
    let status = if active {
        lease.status.unwrap_or_else(|| "ACTIVE".to_string())
    } else if lease.status.as_deref() == Some("RELEASED") {
        "RELEASED".to_string()
    } else {
        "EXPIRED".to_string()
    };

    Some(MarketplaceLeaseStatus {
        id: lease.id,
        provider: lease.provider,
        endpoint: lease.endpoint,
        model: lease.model,
        starts_at: lease.starts_at,
        expires_at: lease.expires_at,
        status,
        using_now,
    })
}

pub(crate) async fn load_active_marketplace_lease(lease_id: &str) -> Result<MarketplaceLease> {
    let path = marketplace_leases_path()?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|err| color_eyre::eyre::eyre!("failed to read {}: {err}", path.display()))?;
    let file: MarketplaceLeasesFile = serde_json::from_str(&content)
        .map_err(|err| color_eyre::eyre::eyre!("failed to parse {}: {err}", path.display()))?;
    let lease = file
        .leases
        .into_iter()
        .find(|lease| lease.id == lease_id)
        .ok_or_else(|| color_eyre::eyre::eyre!("lease {lease_id} was not found"))?;

    if lease.auth_token.trim().is_empty() {
        bail!("lease {lease_id} is missing its local auth token");
    }
    if !lease.is_active() {
        bail!("lease {lease_id} is not active or has expired");
    }

    Ok(lease)
}

fn load_marketplace_leases_sync() -> Result<MarketplaceLeasesFile> {
    let path = marketplace_leases_path()?;
    let content = std::fs::read_to_string(&path)
        .map_err(|err| color_eyre::eyre::eyre!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| color_eyre::eyre::eyre!("failed to parse {}: {err}", path.display()))
}

fn marketplace_leases_path() -> Result<PathBuf> {
    if let Some(home) = std::env::var_os("SOLAI_HOME") {
        return Ok(PathBuf::from(home).join(MARKETPLACE_LEASES_FILE));
    }
    let Some(home) = dirs::home_dir() else {
        bail!("could not resolve home directory for ~/.solai");
    };
    Ok(home.join(".solai").join(MARKETPLACE_LEASES_FILE))
}

fn sanitize_provider_key(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    sanitized.trim_matches('-').to_string()
}
