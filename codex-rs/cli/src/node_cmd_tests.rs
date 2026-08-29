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
