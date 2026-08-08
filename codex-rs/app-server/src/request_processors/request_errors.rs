use super::*;

pub(super) fn environment_selection_error(err: SolaiAgentErr) -> JSONRPCErrorError {
    match err {
        SolaiAgentErr::InvalidRequest(message) => invalid_request(message),
        err => internal_error(format!("failed to validate environment selections: {err}")),
    }
}
