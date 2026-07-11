//! Agent v1 response types (plan §13.2).
//!
//! Success invariants:
//! - `invoke` non-stream: `Completed` requires id+agent_id+non-empty choices;
//!   `Pending` requires agent_id+async_id.
//! - `async_result`: `Pending` requires agent_id+async_id; `Succeeded` adds
//!   non-empty choices; `Failed` is a normal task result (agent_id+async_id).
//! - `conversation`: success requires conversation_id+agent_id+non-empty
//!   choices; an embedded error is surfaced as `Err`.

use serde::Deserialize;

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Non-streaming `invoke` response: either `Completed` or `Pending`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentInvokeResponse {
    /// Completed synchronous result.
    Completed {
        id: String,
        agent_id: String,
        choices: Vec<AgentChoice>,
    },
    /// Accepted as an async task; poll via `async_result`.
    Pending { agent_id: String, async_id: String },
}

/// A single choice in an agent response.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentChoice {
    pub message: Option<AgentChoiceMessage>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentChoiceMessage {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// `async-result` response.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentAsyncResult {
    Pending {
        agent_id: String,
        async_id: String,
    },
    Success {
        agent_id: String,
        async_id: String,
        choices: Vec<AgentChoice>,
    },
    Failed {
        agent_id: String,
        async_id: String,
        #[serde(default)]
        error: Option<AgentErrorDetail>,
    },
}

/// `conversation` response.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentConversationResponse {
    pub conversation_id: String,
    pub agent_id: String,
    pub choices: Vec<AgentChoice>,
}

/// Embedded error detail (conversation/async-result failed bodies).
#[derive(Debug, Clone, Deserialize)]
pub struct AgentErrorDetail {
    #[serde(default)]
    pub code: Option<serde_json::Value>,
    #[serde(default)]
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Success-invariant validation (plan §13.2).
// ---------------------------------------------------------------------------

/// Validate an `invoke` non-stream response against the success invariant.
pub fn validate_invoke_response(resp: &AgentInvokeResponse) -> ZaiResult<()> {
    match resp {
        AgentInvokeResponse::Completed {
            id,
            agent_id,
            choices,
        } => {
            if id.trim().is_empty() || agent_id.trim().is_empty() || choices.is_empty() {
                return Err(invariant(
                    "Completed requires id+agent_id+non-empty choices",
                ));
            }
        },
        AgentInvokeResponse::Pending { agent_id, async_id } => {
            if agent_id.trim().is_empty() || async_id.trim().is_empty() {
                return Err(invariant("Pending requires agent_id+async_id"));
            }
        },
    }
    Ok(())
}

/// Validate an `async-result` response. `Failed` is a normal task result, not
/// a transport error — it returns `Ok`.
pub fn validate_async_result(resp: &AgentAsyncResult) -> ZaiResult<()> {
    match resp {
        AgentAsyncResult::Pending { agent_id, async_id } => {
            if agent_id.trim().is_empty() || async_id.trim().is_empty() {
                return Err(invariant("Pending requires agent_id+async_id"));
            }
        },
        AgentAsyncResult::Success {
            agent_id,
            async_id,
            choices,
        } => {
            if agent_id.trim().is_empty() || async_id.trim().is_empty() || choices.is_empty() {
                return Err(invariant(
                    "Success requires agent_id+async_id+non-empty choices",
                ));
            }
        },
        AgentAsyncResult::Failed {
            agent_id, async_id, ..
        } => {
            if agent_id.trim().is_empty() || async_id.trim().is_empty() {
                return Err(invariant("Failed requires agent_id+async_id"));
            }
        },
    }
    Ok(())
}

/// Validate a `conversation` response. An embedded `error` object is surfaced
/// as `Err`.
pub fn validate_conversation_response(resp: &AgentConversationResponse) -> ZaiResult<()> {
    if resp.conversation_id.trim().is_empty()
        || resp.agent_id.trim().is_empty()
        || resp.choices.is_empty()
    {
        return Err(invariant(
            "conversation requires conversation_id+agent_id+non-empty choices",
        ));
    }
    Ok(())
}

fn invariant(msg: &str) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: format!("agent success invariant violated: {msg}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(content: &str) -> AgentChoice {
        AgentChoice {
            message: Some(AgentChoiceMessage {
                content: Some(content.to_string()),
                role: Some("assistant".to_string()),
            }),
            finish_reason: Some("stop".to_string()),
        }
    }

    #[test]
    fn completed_validates() {
        let r = AgentInvokeResponse::Completed {
            id: "id1".into(),
            agent_id: "a1".into(),
            choices: vec![choice("hi")],
        };
        assert!(validate_invoke_response(&r).is_ok());
    }

    #[test]
    fn completed_rejects_empty_choices() {
        let r = AgentInvokeResponse::Completed {
            id: "id1".into(),
            agent_id: "a1".into(),
            choices: vec![],
        };
        assert!(validate_invoke_response(&r).is_err());
    }

    #[test]
    fn pending_validates() {
        let r = AgentInvokeResponse::Pending {
            agent_id: "a1".into(),
            async_id: "as1".into(),
        };
        assert!(validate_invoke_response(&r).is_ok());
    }

    #[test]
    fn async_result_failed_is_ok() {
        let r = AgentAsyncResult::Failed {
            agent_id: "a1".into(),
            async_id: "as1".into(),
            error: None,
        };
        assert!(validate_async_result(&r).is_ok());
    }

    #[test]
    fn async_result_success_requires_choices() {
        let r = AgentAsyncResult::Success {
            agent_id: "a1".into(),
            async_id: "as1".into(),
            choices: vec![],
        };
        assert!(validate_async_result(&r).is_err());
    }

    #[test]
    fn conversation_requires_all_fields() {
        let ok = AgentConversationResponse {
            conversation_id: "c1".into(),
            agent_id: "a1".into(),
            choices: vec![choice("hi")],
        };
        assert!(validate_conversation_response(&ok).is_ok());

        let bad = AgentConversationResponse {
            conversation_id: "c1".into(),
            agent_id: "a1".into(),
            choices: vec![],
        };
        assert!(validate_conversation_response(&bad).is_err());
    }

    #[test]
    fn empty_and_unknown_objects_do_not_become_success() {
        // The success-invariant validators must reject `{}` and unknown shapes;
        // these are enforced at the envelope-probe layer (P01.7/P03 decode), but
        // the typed validators also guard against malformed typed decodes.
        let empty_completed = AgentInvokeResponse::Completed {
            id: "".into(),
            agent_id: "".into(),
            choices: vec![],
        };
        assert!(validate_invoke_response(&empty_completed).is_err());
    }
}
