use std::fmt::Debug;
use serde::ser::{Error, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};
use validator::*;
use super::traits::*;
use super::model_validate::validate_json_schema;



#[derive(Debug, Clone, Validate, Serialize)]
pub struct ChatBody<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    pub model: N,

    pub messages: Vec<M>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 1.0))]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 98304))]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,

    // tool_choice: enum<string>, but we don't need it for now

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 6, max = 128))]
    pub user_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 1))]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

// 为支持 thinking 的模型提供专门的方法
impl<N, M> ChatBody<N, M>
where
    N: ModelName + ThinkEnable,
    (N, M): Bounded,
{
    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }
}



#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Clone)]
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
    pub fn new(messages: TextMessage) -> Self {
        Self {
            messages: vec![messages],
        }
    }

    // Add method for internal Vec to discourage exposing Vec in API inputs
    pub fn add_message(mut self, msg: TextMessage) -> Self {
        self.messages.push(msg);
        self
    }
}

impl Debug for TextMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextMessages")
            .field("messages", &self.messages)
            .finish()
    }
}

/// Represents a chat message in the system.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum TextMessage {
    User(TextUserMessage),
    Assistant(TextAssistantMessage),
    System(TextSystemMessage),
    Tool(TextToolMessage),
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct TextAssistantMessage {
    role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    id: String,
    type_: ToolCallType,
    function: Option<FunctionParams>,
}

impl Serialize for ToolCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        // 验证：当 type_ 为 Function 时，function 不能为空
        if matches!(self.type_, ToolCallType::Function) && self.function.is_none() {
            return Err(S::Error::custom(
                "function field is required when type is 'function'"
            ));
        }

        let mut state = serializer.serialize_struct("ToolCall", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", &self.type_)?;

        // 根据类型决定是否序列化 function 字段
        match self.type_ {
            ToolCallType::Function => {
                // Function 类型时，function 字段必须存在（上面已验证）
                state.serialize_field("function", &self.function)?;
            }
            _ => {
                // 其他类型时，只有当 function 存在时才序列化
                if self.function.is_some() {
                    state.serialize_field("function", &self.function)?;
                }
            }
        }

        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallType {
    Function,
    WebSearch,
    Retrieval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParams {
    name: String,
    arguments: String,
}

impl ToolCall {
    pub fn new_function(id: impl Into<String>, function: FunctionParams) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Function,
            function: Some(function),
        }
    }

    pub fn new_web_search(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::WebSearch,
            function: None,
        }
    }

    pub fn new_retrieval(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Retrieval,
            function: None,
        }
    }
}

impl FunctionParams {
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

impl TextAssistantMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            tool_calls: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TextToolMessage {
    role: Role,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl TextToolMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: None,
        }
    }
    pub fn with_tool_call_id(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum ThinkingType{
    Enable,
    Disable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Tools {
    #[serde(rename = "function_call")]
    FunctionCall {
        function_call: Vec<FunctionCall>
    },
    #[serde(rename = "retrieval")]
    Retrieval {
        retrieval: Vec<Retrieval>
    },
    #[serde(rename = "web_search")]
    WebSearch {
        web_search: Vec<WebSearch>
    },
    #[serde(rename = "mcp")]
    MCP {
        mcp: Vec<MCP>
    },
}

#[derive(Debug, Clone, Serialize, Validate)]
pub struct FunctionCall{
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    pub description: String,

    #[validate(custom(function = "validate_json_schema"))]
    pub parameters: String,
}

impl FunctionCall{
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: impl Into<String>) -> Self{
        Self{
            name: name.into(),
            description: description.into(),
            parameters: parameters.into(),
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize)]
pub struct Retrieval{

}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct WebSearch{

}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MCP{

}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ResponseFormat{
    Text,
    JsonObject,
}