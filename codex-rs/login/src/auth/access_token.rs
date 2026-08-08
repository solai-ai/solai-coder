const PERSONAL_ACCESS_TOKEN_PREFIX: &str = "at-";

pub(super) enum SolaiAgentAccessToken<'a> {
    PersonalAccessToken(&'a str),
    AgentIdentityJwt(&'a str),
}

pub(super) fn classify_codex_access_token(access_token: &str) -> SolaiAgentAccessToken<'_> {
    if access_token.starts_with(PERSONAL_ACCESS_TOKEN_PREFIX) {
        SolaiAgentAccessToken::PersonalAccessToken(access_token)
    } else {
        SolaiAgentAccessToken::AgentIdentityJwt(access_token)
    }
}

#[cfg(test)]
#[path = "access_token_tests.rs"]
mod tests;
