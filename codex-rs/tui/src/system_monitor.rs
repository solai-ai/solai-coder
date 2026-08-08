use crate::tui::FrameRequester;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use tokio::task::JoinHandle;
use url::Url;

const MONITOR_PORT: u16 = 9898;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_millis(800);
const MONITOR_LABEL_STYLE: Style = Style::new().fg(Color::White);
const MONITOR_SEPARATOR_STYLE: Style = Style::new().fg(Color::Gray);

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetricsSnapshot {
    cpu: Option<CpuMetrics>,
    ram: Option<MemoryMetrics>,
    swap: Option<MemoryMetrics>,
    gpu: Option<GpuMetrics>,
    ollama: Option<OllamaMetrics>,
}

#[derive(Clone, Debug, Deserialize)]
struct CpuMetrics {
    usage: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryMetrics {
    used_bytes: u64,
    total_bytes: u64,
    usage_percent: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GpuMetrics {
    vendor: Option<String>,
    model: Option<String>,
    usage_percent: Option<f64>,
    temperature_celsius: Option<f64>,
    vram: Option<VramMetrics>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VramMetrics {
    used_bytes: Option<u64>,
    total_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
struct OllamaMetrics {
    #[serde(default)]
    running: Vec<OllamaProcess>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OllamaProcess {
    context: Option<u64>,
    context_length: Option<u64>,
    cpu_percent: Option<f64>,
    gpu_percent: Option<f64>,
}

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString, Eq, PartialEq)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum MonitorLineItem {
    Cpu,
    Ram,
    Swap,
    Gpu,
    GpuModel,
    Vram,
    GpuTemp,
    Status,
    Context,
    GpuCpu,
}

impl MonitorLineItem {
    pub(crate) fn description(self) -> &'static str {
        match self {
            MonitorLineItem::Cpu => "CPU usage",
            MonitorLineItem::Ram => "System RAM usage",
            MonitorLineItem::Swap => "Swap usage",
            MonitorLineItem::Gpu => "GPU usage",
            MonitorLineItem::GpuModel => "GPU vendor and model",
            MonitorLineItem::Vram => "GPU VRAM usage",
            MonitorLineItem::GpuTemp => "GPU temperature",
            MonitorLineItem::Status => "Ollama runtime status",
            MonitorLineItem::Context => "Ollama context size and context limit",
            MonitorLineItem::GpuCpu => "Ollama GPU versus CPU split",
        }
    }
}

pub(crate) fn parse_monitor_line_items(
    ids: impl IntoIterator<Item = String>,
) -> (Vec<MonitorLineItem>, Vec<String>) {
    let mut invalid = Vec::new();
    let mut invalid_seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    for id in ids {
        match id.parse::<MonitorLineItem>() {
            Ok(item) => items.push(item),
            Err(_) => {
                if invalid_seen.insert(id.clone()) {
                    invalid.push(format!(r#""{id}""#));
                }
            }
        }
    }
    (items, invalid)
}

pub(crate) struct SystemMonitor {
    snapshot: Arc<RwLock<Option<MetricsSnapshot>>>,
    task: Option<JoinHandle<()>>,
}

impl SystemMonitor {
    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(None)),
            task: None,
        }
    }

    pub(crate) fn start(provider_base_url: Option<&str>, frame_requester: FrameRequester) -> Self {
        let endpoint = provider_base_url.and_then(monitor_endpoint);
        let snapshot = Arc::new(RwLock::new(None));
        let task = endpoint.as_ref().map(|endpoint| {
            let endpoint = endpoint.clone();
            let snapshot = Arc::clone(&snapshot);
            tokio::spawn(async move {
                let client = match reqwest::Client::builder().timeout(REQUEST_TIMEOUT).build() {
                    Ok(client) => client,
                    Err(err) => {
                        tracing::debug!(%err, "failed to create SOLAI Monitor client");
                        return;
                    }
                };
                loop {
                    if let Ok(response) = client.get(&endpoint).send().await
                        && let Ok(next) = response.error_for_status()
                        && let Ok(next) = next.json::<MetricsSnapshot>().await
                    {
                        if let Ok(mut current) = snapshot.write() {
                            *current = Some(next);
                        }
                        frame_requester.schedule_frame();
                    }
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            })
        });
        Self { snapshot, task }
    }

    pub(crate) fn footer_line(
        &self,
        items: &[MonitorLineItem],
        use_colors: bool,
    ) -> Option<Line<'static>> {
        let snapshot = self.snapshot.read().ok().and_then(|value| value.clone())?;
        let mut spans = Vec::new();
        let mut pushed_any = false;
        for item in items {
            let Some(segment) = monitor_segment(&snapshot, *item) else {
                continue;
            };
            if pushed_any {
                spans.push(Span::styled(" · ", MONITOR_SEPARATOR_STYLE));
            }
            pushed_any = true;
            spans.push(Span::styled(segment.label, MONITOR_LABEL_STYLE));
            spans.push(Span::styled(": ", MONITOR_SEPARATOR_STYLE));
            if use_colors {
                spans.push(Span::styled(
                    segment.value,
                    Style::default().fg(segment.color),
                ));
            } else {
                spans.push(Span::from(segment.value));
            }
        }
        pushed_any.then(|| Line::from(spans))
    }
}

impl Drop for SystemMonitor {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

fn monitor_endpoint(provider_base_url: &str) -> Option<String> {
    let mut url = Url::parse(provider_base_url).ok()?;
    url.set_port(Some(MONITOR_PORT)).ok()?;
    url.set_path("/metrics");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.into())
}

struct MonitorSegment {
    label: &'static str,
    value: String,
    color: Color,
}

fn monitor_segment(snapshot: &MetricsSnapshot, item: MonitorLineItem) -> Option<MonitorSegment> {
    match item {
        MonitorLineItem::Cpu => snapshot.cpu.as_ref().map(|cpu| MonitorSegment {
            label: "CPU",
            value: format!("{:.0}%", cpu.usage),
            color: color_for_ratio(cpu.usage / 100.0),
        }),
        MonitorLineItem::Ram => snapshot.ram.as_ref().map(|ram| MonitorSegment {
            label: "RAM",
            value: format_bytes(ram.used_bytes, ram.total_bytes),
            color: color_for_ratio(ram.usage_percent / 100.0),
        }),
        MonitorLineItem::Swap => snapshot.swap.as_ref().map(|swap| MonitorSegment {
            label: "SWAP",
            value: format_bytes(swap.used_bytes, swap.total_bytes),
            color: color_for_ratio(swap.usage_percent / 100.0),
        }),
        MonitorLineItem::Gpu => snapshot.gpu.as_ref().and_then(|gpu| {
            gpu.usage_percent.map(|usage| MonitorSegment {
                label: "GPU",
                value: format!("{usage:.0}%"),
                color: color_for_ratio(usage / 100.0),
            })
        }),
        MonitorLineItem::GpuModel => snapshot.gpu.as_ref().map(|gpu| MonitorSegment {
            label: "GPU MODEL",
            value: match (gpu.vendor.as_deref(), gpu.model.as_deref()) {
                (Some(vendor), Some(model)) if !vendor.is_empty() && !model.is_empty() => {
                    format!("{vendor} {model}")
                }
                (Some(vendor), _) if !vendor.is_empty() => vendor.to_string(),
                (_, Some(model)) if !model.is_empty() => model.to_string(),
                _ => "n/a".to_string(),
            },
            color: Color::LightMagenta,
        }),
        MonitorLineItem::Vram => snapshot.gpu.as_ref().and_then(|gpu| {
            gpu.vram.as_ref().and_then(|vram| {
                let (Some(used), Some(total)) = (vram.used_bytes, vram.total_bytes) else {
                    return None;
                };
                Some(MonitorSegment {
                    label: "VRAM",
                    value: format_bytes(used, total),
                    color: color_for_ratio(if total == 0 {
                        0.0
                    } else {
                        used as f64 / total as f64
                    }),
                })
            })
        }),
        MonitorLineItem::GpuTemp => snapshot.gpu.as_ref().and_then(|gpu| {
            gpu.temperature_celsius.map(|temp| MonitorSegment {
                label: "GPU TEMP",
                value: format!("{temp:.0} C"),
                color: color_for_ratio(temp / 100.0),
            })
        }),
        MonitorLineItem::Status => snapshot
            .ollama
            .as_ref()
            .and_then(|ollama| ollama.running.first())
            .map(|process| MonitorSegment {
                label: "STATUS",
                value: if process.context.is_some() || process.context_length.is_some() {
                    "ACTIVE".to_string()
                } else {
                    "RUNNING".to_string()
                },
                color: Color::LightGreen,
            }),
        MonitorLineItem::Context => snapshot
            .ollama
            .as_ref()
            .and_then(|ollama| ollama.running.first())
            .map(|process| MonitorSegment {
                label: "CTX",
                value: format!(
                    "{} / {}",
                    process
                        .context
                        .map_or_else(|| "n/a".to_string(), format_compact_count),
                    process
                        .context_length
                        .map_or_else(|| "n/a".to_string(), format_compact_count),
                ),
                color: Color::LightCyan,
            }),
        MonitorLineItem::GpuCpu => snapshot
            .ollama
            .as_ref()
            .and_then(|ollama| ollama.running.first())
            .map(|process| MonitorSegment {
                label: "GPU x CPU",
                value: format!(
                    "GPU {} / CPU {}",
                    process
                        .gpu_percent
                        .map_or_else(|| "n/a".to_string(), |value| format!("{value:.0}%")),
                    process
                        .cpu_percent
                        .map_or_else(|| "n/a".to_string(), |value| format!("{value:.0}%")),
                ),
                color: color_for_ratio(process.gpu_percent.unwrap_or(0.0) / 100.0),
            }),
    }
}

fn color_for_ratio(ratio: f64) -> Color {
    if ratio >= 0.9 {
        Color::LightRed
    } else if ratio >= 0.7 {
        Color::LightYellow
    } else {
        Color::LightGreen
    }
}

fn format_compact_count(value: u64) -> String {
    const THOUSAND: f64 = 1000.0;
    const MILLION: f64 = 1000.0 * 1000.0;
    if value >= MILLION as u64 {
        format!("{:.1}M", value as f64 / MILLION)
    } else if value >= THOUSAND as u64 {
        format!("{:.1}k", value as f64 / THOUSAND)
    } else {
        value.to_string()
    }
}

fn format_bytes(used: u64, total: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1}/{:.1}G", used as f64 / GIB, total as f64 / GIB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_endpoint_uses_provider_host_and_monitor_port() {
        assert_eq!(
            monitor_endpoint("http://192.168.1.20:11434/v1"),
            Some("http://192.168.1.20:9898/metrics".to_string())
        );
    }
}
