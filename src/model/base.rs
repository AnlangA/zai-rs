use serde::ser::{Error, SerializeStruct};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessages {
    pub messages: Vec<ChatMessage>,
}

impl ChatMessages {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    // Add method for internal Vec to discourage exposing Vec in API inputs
    pub fn add_message(mut self, msg: ChatMessage) -> Self {
        self.messages.push(msg);
        self
    }
}

/// Represents a chat message in the system.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub user_message: Option<UserMessage>,
    pub assistant_message: Option<AssistantMessage>,
    pub system_message: Option<SystemMessage>,
    pub tool_message: Option<ToolMessage>,
}

/// Serialize a ChatMessage into a JSON string.
impl Serialize for ChatMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let count = self.user_message.is_some() as u8
            + self.assistant_message.is_some() as u8
            + self.system_message.is_some() as u8
            + self.tool_message.is_some() as u8;

        if count == 0 {
            return Err(S::Error::custom(
                "ChatMessage must have at least one non-none message",
            ));
        }
        if count > 1 {
            // 为避免键冲突，序列化时要求仅有一个消息被设置
            return Err(S::Error::custom(
                "ChatMessage must have exactly one message when flatten-serializing",
            ));
        }

        if let Some(ref m) = self.user_message {
            return m.serialize(serializer);
        }
        if let Some(ref m) = self.assistant_message {
            return m.serialize(serializer);
        }
        if let Some(ref m) = self.system_message {
            return m.serialize(serializer);
        }
        if let Some(ref m) = self.tool_message {
            return m.serialize(serializer);
        }

        unreachable!();
    }
}

impl ChatMessage {
    pub fn user(user: UserMessage) -> Self {
        Self {
            user_message: Some(user),
            assistant_message: None,
            system_message: None,
            tool_message: None,
        }
    }
    pub fn assistant(assistant: AssistantMessage) -> Self {
        Self {
            user_message: None,
            assistant_message: Some(assistant),
            system_message: None,
            tool_message: None,
        }
    }
    pub fn system(system: SystemMessage) -> Self {
        Self {
            user_message: None,
            assistant_message: None,
            system_message: Some(system),
            tool_message: None,
        }
    }
    pub fn tool(tool: ToolMessage) -> Self {
        Self {
            user_message: None,
            assistant_message: None,
            system_message: None,
            tool_message: Some(tool),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserMessage {
    role: Role,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl UserMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
        }
    }
    pub fn with_tool_call_id(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AssistantMessage {
    role: Role,
    content: String,
}

impl AssistantMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMessage {
    role: Role,
    content: String,
}

impl SystemMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolMessage {
    role: Role,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

impl Serialize for ToolMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields = 1;
        let include_content = match self.content {
            Some(_) => true,
            None => false,
        };
        let include_tool_calls = match self.tool_calls {
            Some(_) => true,
            None => false,
        };

        if !include_content && !include_tool_calls {
            return Err(S::Error::custom(
                "ToolMessage must have at least one of 'content' or 'tool_calls'",
            ));
        }

        if include_content {
            fields += 1;
        }
        if include_tool_calls {
            fields += 1;
        }

        let mut st = serializer.serialize_struct("ToolMessage", fields)?;
        st.serialize_field("role", &self.role)?;
        if include_content {
            st.serialize_field("content", &self.content)?;
        }
        if include_tool_calls {
            st.serialize_field("tool_calls", &self.tool_calls)?;
        }
        st.end()
    }
}

impl ToolMessage {
    // new: no required args for ToolMessage
    pub fn new() -> Self {
        Self {
            role: Role::Tool,
            content: None,
            tool_calls: None,
        }
    }

    // with_*: optional setters
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    // add_*: for internal Vec
    pub fn add_tool_call(mut self, call: ToolCall) -> Self {
        if let Some(ref mut v) = self.tool_calls {
            v.push(call);
        } else {
            self.tool_calls = Some(vec![call]);
        }
        self
    }

    pub fn with_both(mut self, content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        self.content = Some(content.into());
        self.tool_calls = Some(tool_calls);
        self
    }
}
#[derive(Debug, Clone)]
pub struct ToolCall {
    id: String,
    type_: ToolType,
    function: Option<FunctionCall>,
}

impl Serialize for ToolCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // id 和 type 固定存在
        let mut fields = 2;
        let include_function = match self.type_ {
            ToolType::Function => {
                if self.function.is_none() {
                    return Err(S::Error::custom(
                        "ToolCall.function must be present when type is 'function'",
                    ));
                }
                true
            }
            _ => false,
        };
        if include_function {
            fields += 1;
        }

        let mut st = serializer.serialize_struct("ToolCall", fields)?;
        st.serialize_field("id", &self.id)?;
        st.serialize_field("type", &self.type_)?;
        if include_function {
            st.serialize_field("function", self.function.as_ref().unwrap())?;
        }
        st.end()
    }
}

impl ToolCall {
    // new: only required fields
    pub fn new(id: impl Into<String>, type_: ToolType) -> Self {
        Self {
            id: id.into(),
            type_,
            function: None,
        }
    }

    // with_*: optional parts
    pub fn with_function(mut self, name: impl Into<String>, arguments: impl Into<String>) -> Self {
        self.function = Some(FunctionCall {
            name: name.into(),
            arguments: arguments.into(),
        });
        self
    }

    pub fn with_function_call(mut self, function: FunctionCall) -> Self {
        self.function = Some(function);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Function,
    WebSearch,
    Retrieval,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionCall {
    name: String,
    arguments: String,
}

impl FunctionCall {
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

// Builders to avoid Vec in API inputs while keeping serialization unchanged.

#[cfg(test)]
mod tests {
    use super::{
        AssistantMessage, ChatMessage, ChatMessages, SystemMessage, ToolCall, ToolMessage,
        ToolType, UserMessage,
    };
    use serde_json::{json, to_value};

    #[test]
    fn chat_message_user_flatten_ok() {
        let um = UserMessage::new("hi");
        let msg = ChatMessage::user(um);
        let v = to_value(&msg).unwrap();
        assert_eq!(v, json!({"role":"user","content":"hi"}));
    }

    #[test]
    fn chat_message_multiple_error() {
        let msg = ChatMessage {
            user_message: Some(UserMessage::new("hi")),
            assistant_message: Some(AssistantMessage::new("hello")),
            system_message: None,
            tool_message: None,
        };
        let err = serde_json::to_string(&msg).unwrap_err().to_string();
        assert!(err.contains("exactly one message"), "{err}");
    }

    #[test]
    fn chat_message_none_error() {
        let msg = ChatMessage {
            user_message: None,
            assistant_message: None,
            system_message: None,
            tool_message: None,
        };
        let err = serde_json::to_string(&msg).unwrap_err().to_string();
        assert!(err.contains("at least one non-none message"), "{err}");
    }

    #[test]
    fn toolcall_function_required_error() {
        let tc = ToolCall {
            id: "id1".into(),
            type_: ToolType::Function,
            function: None,
        };
        let err = serde_json::to_string(&tc).unwrap_err().to_string();
        assert!(err.contains("must be present"), "{err}");
    }

    #[test]
    fn toolcall_non_function_ok_omits_function() {
        let tc = ToolCall::new("id2", ToolType::WebSearch);
        let v = to_value(&tc).unwrap();
        assert_eq!(v, json!({"id":"id2","type":"web_search"}));
    }

    #[test]
    fn toolmessage_validation() {
        // both None -> error
        let tm = ToolMessage::new();
        let err = serde_json::to_string(&tm).unwrap_err().to_string();
        assert!(
            err.contains("at least one of 'content' or 'tool_calls'"),
            "{err}"
        );

        // content only -> ok
        let tm = ToolMessage::new().with_content("hi");
        let v = to_value(&tm).unwrap();
        assert_eq!(v, json!({"role":"tool","content":"hi"}));

        // tool_calls only -> ok
        let tc = ToolCall::new("id3", ToolType::Retrieval);
        let tm = ToolMessage::new().with_tool_calls(vec![tc]);
        let v = to_value(&tm).unwrap();
        assert_eq!(
            v,
            json!({"role":"tool","tool_calls":[{"id":"id3","type":"retrieval"}]})
        );

        // both -> ok
        let tc =
            ToolCall::new("id4", ToolType::Function).with_function("search", r#"{"q":"rust"}"#);
        let tm = ToolMessage::new().with_both("tools:", vec![tc]);
        let v = to_value(&tm).unwrap();
        assert_eq!(
            v,
            json!({
                "role":"tool",
                "content":"tools:",
                "tool_calls":[{"id":"id4","type":"function","function":{"name":"search","arguments":"{\"q\":\"rust\"}"}}]
            })
        );
    }

    #[test]
    fn chat_messages_serialize_no_builder() {
        let cms = ChatMessages::new()
            .add_message(ChatMessage::system(SystemMessage::new("sys")))
            .add_message(ChatMessage::user(UserMessage::new("hi")))
            .add_message(ChatMessage::assistant(AssistantMessage::new("hello")))
            .add_message(ChatMessage::tool(ToolMessage::new().with_content("tools:")));

        let v = to_value(&cms).unwrap();
        assert_eq!(
            v,
            json!({
                "messages": [
                    {"role":"system","content":"sys"},
                    {"role":"user","content":"hi"},
                    {"role":"assistant","content":"hello"},
                    {"role":"tool","content":"tools:"}
                ]
            })
        );
    }
}
