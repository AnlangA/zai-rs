use serde::ser::{Error, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};
use super::traits::*;

#[derive(Debug, Clone, Serialize)]
pub struct ChatBody<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    pub model: N,
    pub messages: Vec<M>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone)]
pub struct TextMessages {
    pub messages: Vec<TextMessage>,
}

impl Serialize for TextMessages {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.messages.is_empty() {
            return Err(S::Error::custom(
                "TextMessages must have at least one message",
            ));
        }

        let mut st = serializer.serialize_struct("TextMessages", 1)?;
        st.serialize_field("messages", &self.messages)?;
        st.end()
    }
}

impl TextMessages {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    // Add method for internal Vec to discourage exposing Vec in API inputs
    pub fn add_message(mut self, msg: TextMessage) -> Self {
        self.messages.push(msg);
        self
    }
}

/// Represents a chat message in the system.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "role", content = "content")]
#[serde(rename_all = "lowercase")]
pub enum TextMessage {
    User(TextUserMessage),
    Assistant(TextAssistantMessage),
    System(TextSystemMessage),
    Tool(TextToolMessage),
}

/// Serialize a TextMessage into a JSON string.
impl Serialize for TextMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TextMessage::User(msg) => msg.serialize(serializer),
            TextMessage::Assistant(msg) => msg.serialize(serializer),
            TextMessage::System(msg) => msg.serialize(serializer),
            TextMessage::Tool(msg) => msg.serialize(serializer),
        }
    }
}

impl TextMessage {
    pub fn user(user: TextUserMessage) -> Self {
        TextMessage::User(user)
    }

    pub fn assistant(assistant: TextAssistantMessage) -> Self {
        TextMessage::Assistant(assistant)
    }

    pub fn system(system: TextSystemMessage) -> Self {
        TextMessage::System(system)
    }

    pub fn tool(tool: TextToolMessage) -> Self {
        TextMessage::Tool(tool)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextUserMessage {
    role: Role,
    content: String,
}

impl TextUserMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSystemMessage {
    role: Role,
    content: String,
}

impl TextSystemMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAssistantMessage {
    role: Role,
    content: String,
}

impl TextAssistantMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextToolMessage {
    role: Role,
    content: String,
    tool_call_id: String,
}