use pretty_assertions::assert_eq;

use super::*;

#[test]
fn reads_assistant_content_from_completed_job() {
    let detail = JobDetailResponse {
        job: JobResponse {
            job_id: "job-1".to_string(),
            status: "completed".to_string(),
            extra: BTreeMap::new(),
        },
        result: Some(JobResult {
            output: serde_json::json!({
                "message": {"role": "assistant", "content": "SOLAI_NODE_OK"}
            }),
            extra: BTreeMap::new(),
        }),
        error: None,
        extra: BTreeMap::new(),
    };

    assert_eq!(assistant_content(&detail).unwrap(), "SOLAI_NODE_OK");
}

#[test]
fn normalizes_node_base_url() {
    let client = NodeClient::new(" http://127.0.0.1:9876/ ".to_string(), None).unwrap();

    assert_eq!(
        client.endpoint("/v1/health"),
        "http://127.0.0.1:9876/v1/health"
    );
}

#[test]
fn serializes_runtime_mode_for_node_contract() {
    let request = admin::UpdateRuntimeModeRequest {
        mode: admin::RuntimeModeArg::Draining,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"mode": "draining"})
    );
}

#[test]
fn serializes_bounded_event_filters() {
    let client = NodeClient::new("http://127.0.0.1:9876".to_string(), None).unwrap();
    let query = admin::ListEventsQuery {
        kind: Some("job_completed".to_string()),
        provider_id: Some("provider-a".to_string()),
        job_id: None,
        limit: 25,
        offset: 5,
    };
    let request = client
        .http
        .get(client.endpoint("/v1/events"))
        .query(&query)
        .build()
        .unwrap();

    assert_eq!(
        request.url().query(),
        Some("kind=job_completed&provider_id=provider-a&limit=25&offset=5")
    );
}

#[test]
fn serializes_bounded_job_filters() {
    let client = NodeClient::new("http://127.0.0.1:9876".to_string(), None).unwrap();
    let query = jobs::ListJobsQuery {
        status: Some("completed".to_string()),
        workload: Some("chat_complete".to_string()),
        provider_id: None,
        tenant_id: Some("tenant-a".to_string()),
        model: None,
        limit: 25,
        offset: 5,
    };
    let request = client
        .http
        .get(client.endpoint("/v1/jobs"))
        .query(&query)
        .build()
        .unwrap();

    assert_eq!(
        request.url().query(),
        Some("status=completed&workload=chat_complete&tenant_id=tenant-a&limit=25&offset=5")
    );
}

#[test]
fn serializes_bounded_provider_filters() {
    let client = NodeClient::new("http://127.0.0.1:9876".to_string(), None).unwrap();
    let query = providers::ListProvidersQuery {
        status: Some("available".to_string()),
        model: Some("gemma3:4b".to_string()),
        has_capacity: Some(true),
        at_capacity: Some(false),
        min_available_slots: Some(2),
        sort: Some("available_capacity".to_string()),
        limit: 25,
        offset: 5,
    };
    let request = client
        .http
        .get(client.endpoint("/v1/providers"))
        .query(&query)
        .build()
        .unwrap();

    assert_eq!(
        request.url().query(),
        Some(
            "status=available&model=gemma3%3A4b&has_capacity=true&at_capacity=false&min_available_slots=2&sort=available_capacity&limit=25&offset=5"
        )
    );
}

#[test]
fn validates_provider_operator_inputs() {
    assert_eq!(providers::parse_positive_decimal("1.25").unwrap(), "1.25");
    assert!(providers::parse_positive_decimal("0").is_err());
    assert_eq!(providers::parse_hh_mm("22:00").unwrap(), "22:00");
    assert!(providers::parse_hh_mm("24:00").is_err());
}

#[test]
fn heartbeat_omits_status_when_only_capacity_changes() {
    let request = providers::ProviderHeartbeatRequest {
        status: None,
        clear_capacity: false,
        capacity: Some(4),
        active_jobs: Some(1),
        models: None,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"capacity": 4, "active_jobs": 1})
    );
}

#[test]
fn heartbeat_serializes_explicit_status_changes() {
    let request = providers::ProviderHeartbeatRequest {
        status: Some(providers::ProviderStatusArg::Unavailable),
        clear_capacity: false,
        capacity: None,
        active_jobs: None,
        models: None,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"status": "unavailable"})
    );
}

#[test]
fn heartbeat_serializes_capacity_clearing() {
    let request = providers::ProviderHeartbeatRequest {
        status: None,
        clear_capacity: true,
        capacity: None,
        active_jobs: None,
        models: None,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"clear_capacity": true})
    );
}

#[test]
fn validates_heartbeat_capacity_before_sending() {
    assert!(providers::validate_heartbeat_capacity(Some(4), Some(4), false).is_ok());
    assert!(providers::validate_heartbeat_capacity(Some(4), Some(5), false).is_err());
    assert!(providers::validate_heartbeat_capacity(None, None, true).is_ok());
    assert!(providers::validate_heartbeat_capacity(Some(4), None, true).is_err());
}
