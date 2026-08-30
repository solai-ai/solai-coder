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
    let request = UpdateRuntimeModeRequest {
        mode: RuntimeModeArg::Draining,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"mode": "draining"})
    );
}

#[test]
fn serializes_bounded_event_filters() {
    let client = NodeClient::new("http://127.0.0.1:9876".to_string(), None).unwrap();
    let query = ListEventsQuery {
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
