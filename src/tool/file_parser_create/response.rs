//! File-parser task-creation response.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Response returned after creating a parser task.
#[derive(Debug, Clone, Serialize)]
pub struct FileParserCreateResponse {
    /// Whether the provider reports successful task creation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Provider message, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Unique parsing-task identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Deserialize)]
struct FileParserCreateResponseWire {
    success: Option<bool>,
    message: Option<String>,
    task_id: Option<String>,
}

impl<'de> Deserialize<'de> for FileParserCreateResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FileParserCreateResponseWire::deserialize(deserializer)?;
        if wire.success.is_none() && wire.message.is_none() && wire.task_id.is_none() {
            return Err(D::Error::custom(
                "file-parser response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            success: wire.success,
            message: wire.message,
            task_id: wire.task_id,
        })
    }
}

impl FileParserCreateResponse {
    /// Return whether the response contains a usable task identifier.
    pub fn is_success(&self) -> bool {
        self.success != Some(false)
            && self
                .task_id
                .as_deref()
                .is_some_and(|task_id| !task_id.trim().is_empty())
    }

    /// Borrow the task ID when returned.
    pub fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_optional_schema_fields_but_rejects_empty_success() {
        assert!(serde_json::from_str::<FileParserCreateResponse>("{}").is_err());
        assert!(serde_json::from_str::<FileParserCreateResponse>(r#"{"success":null}"#).is_err());
        let response: FileParserCreateResponse =
            serde_json::from_str(r#"{"success":true,"task_id":"task-1"}"#).unwrap();
        assert!(response.is_success());
        assert_eq!(response.task_id(), Some("task-1"));
    }
}
