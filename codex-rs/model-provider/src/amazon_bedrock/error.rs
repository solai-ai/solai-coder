use codex_api::ApiError;
use codex_protocol::error::SolaiAgentErr;
use http::StatusCode;

pub(super) const BEDROCK_EXPIRED_SIGNATURE_MESSAGE: &str = concat!(
    "Amazon Bedrock rejected the request because its AWS signature has expired. ",
    "Refresh your AWS credentials and retry. If `AWS_BEARER_TOKEN_BEDROCK` is set, ",
    "update or unset it, then restart SolaiAgent",
);

pub(super) fn map_api_error(error: ApiError) -> SolaiAgentErr {
    let mut error = codex_api::map_api_error(error);
    if let SolaiAgentErr::UnexpectedStatus(response) = &mut error
        && response.status == StatusCode::UNAUTHORIZED
        && response.body.contains("Signature expired:")
    {
        response.user_message = Some(BEDROCK_EXPIRED_SIGNATURE_MESSAGE.to_string());
    }
    error
}
