//! Agent v1 response types and explicit validation helpers.
//!
//! Success invariants:
//! - `invoke` non-stream: `Completed` requires id+agent_id+non-empty choices;
//!   `Pending` requires agent_id+async_id.
//! - `async_result`: `Pending` requires agent_id+async_id; `Success` adds
//!   non-empty choices; `Failed` is a normal task result (agent_id+async_id).
//! - `conversation`: success requires conversation_id+agent_id+non-empty
//!   choices.
//!
//! Deserialization alone does not run these checks. Call the corresponding
//! `validate_*` function after decoding a response.

use serde::Deserialize;

use crate::{ZaiError, ZaiResult, client::error::codes};

/// Non-streaming `invoke` response: either `Completed` or `Pending`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentInvokeResponse {
    /// Completed synchronous result.
    Completed {
        /// Invocation identifier.
        id: String,
        /// Agent identifier returned by the service.
        agent_id: String,
        /// Generated choices; validation requires this collection to be non-empty.
        choices: Vec<AgentChoice>,
    },
    /// Accepted as an async task; poll via `async_result`.
    Pending {
        /// Agent identifier returned by the service.
        agent_id: String,
        /// Identifier used to poll the asynchronous result endpoint.
        async_id: String,
    },
}

/// A single choice in an agent response.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentChoice {
    /// Assistant message, when the response includes one.
    pub message: Option<AgentChoiceMessage>,
    /// Service-provided reason that generation stopped.
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Message contained in an [`AgentChoice`].
#[derive(Debug, Clone, Deserialize)]
pub struct AgentChoiceMessage {
    /// Generated textual content, when present.
    #[serde(default)]
    pub content: Option<String>,
    /// Message role reported by the service.
    #[serde(default)]
    pub role: Option<String>,
}

/// `async-result` response.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentAsyncResult {
    /// The asynchronous invocation has not finished.
    Pending {
        /// Agent identifier returned by the service.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
    },
    /// The invocation completed successfully.
    Success {
        /// Agent identifier returned by the service.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
        /// Generated choices; validation requires this collection to be non-empty.
        choices: Vec<AgentChoice>,
    },
    /// The invocation reached a failed terminal state.
    Failed {
        /// Agent identifier returned by the service.
        agent_id: String,
        /// Asynchronous task identifier.
        async_id: String,
        /// Optional failure details supplied by the service.
        #[serde(default)]
        error: Option<AgentErrorDetail>,
    },
}

/// `conversation` response.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentConversationResponse {
    /// Conversation identifier returned by the service.
    pub conversation_id: String,
    /// Agent identifier returned by the service.
    pub agent_id: String,
    /// Generated choices; validation requires this collection to be non-empty.
    pub choices: Vec<AgentChoice>,
}

/// Embedded error detail (conversation/async-result failed bodies).
#[derive(Debug, Clone, Deserialize)]
pub struct AgentErrorDetail {
    /// Open-format service error code.
    #[serde(default)]
    pub code: Option<serde_json::Value>,
    /// Human-readable failure message.
    #[serde(default)]
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Success-invariant validation.
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

/// Validate the required identifiers and choices in a `conversation` response.
///
/// [`AgentConversationResponse`] has no embedded error field; transport-level
/// error-envelope handling, if needed, must happen before deserialization.
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
        // The typed validators guard against malformed values constructed in
        // code as well as values produced by permissive upstream decoding.
        let empty_completed = AgentInvokeResponse::Completed {
            id: "".into(),
            agent_id: "".into(),
            choices: vec![],
        };
        assert!(validate_invoke_response(&empty_completed).is_err());
    }
}
