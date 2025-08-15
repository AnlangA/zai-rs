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

#[derive(Clone, Serialize, Validate)]
pub struct TextMessages {
    #[validate(length(min = 1, max = 1000))]
    pub messages: Vec<TextMessage>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum TextMessage {
    User {
        content: String,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    System {
        content: String,
    },
    Tool {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
    },
}

impl TextMessage {
    pub fn user(content: impl Into<String>) -> Self {
        TextMessage::User {
            content: content.into()
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        TextMessage::Assistant {
            content: Some(content.into()),
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        TextMessage::Assistant {
            content,
            tool_calls,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        TextMessage::System {
            content: content.into()
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        TextMessage::Tool {
            content: content.into(),
            tool_call_id: None,
        }
    }

    pub fn tool_with_id(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        TextMessage::Tool {
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
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
    FunctionCall {
        function_call: Vec<FunctionCall>
    },
    Retrieval {
        retrieval: Vec<Retrieval>
    },
    WebSearch {
        web_search: Vec<WebSearch>
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // 定义测试用的模型类型
    #[derive(Debug, Clone, serde::Serialize)]
    struct TestModel {
        name: String,
    }

    impl TestModel {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
            }
        }
    }

    impl Into<String> for TestModel {
        fn into(self) -> String {
            self.name
        }
    }

    impl ModelName for TestModel {}
    impl ThinkEnable for TestModel {}

    // 实现 Bounded trait
    impl Bounded for (TestModel, TextMessage) {}

    #[test]
    fn test_chatbody_basic_serialization() {
        let model = TestModel::new("gpt-4");
        let messages = vec![
            TextMessage::system("You are a helpful assistant."),
            TextMessage::user("Hello, how are you?"),
            TextMessage::assistant("I'm doing well, thank you! How can I help you today?"),
        ];

        let basic_body = ChatBody {
            model: model.clone(),
            messages: messages.clone(),
            request_id: None,
            thinking: None,
            do_sample: None,
            stream: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            tools: None,
            user_id: None,
            stop: None,
            response_format: None,
        };

        let json = serde_json::to_string_pretty(&basic_body).unwrap();
        println!("基本 ChatBody 序列化结果:");
        println!("{}\n", json);

        // 验证 JSON 包含必要字段
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"role\": \"system\""));
        assert!(json.contains("\"role\": \"user\""));
        assert!(json.contains("\"role\": \"assistant\""));
    }

    #[test]
    fn test_chatbody_full_serialization() {
        let model = TestModel::new("gpt-4");
        let messages = vec![
            TextMessage::system("You are a helpful assistant."),
            TextMessage::user("Hello!"),
        ];

        let function_call = FunctionCall::new(
            "get_weather",
            "Get current weather information",
            r#"{"type": "object", "properties": {"location": {"type": "string"}}}"#
        );

        let tools = Tools::FunctionCall {
            function_call: vec![function_call]
        };

        let full_body = ChatBody {
            model: model.clone(),
            messages: messages.clone(),
            request_id: Some("req_123456".to_string()),
            thinking: Some("Let me think about this...".to_string()),
            do_sample: Some(true),
            stream: Some(false),
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(1000),
            tools: Some(tools),
            user_id: Some("user_789".to_string()),
            stop: Some(vec!["END".to_string()]),
            response_format: Some(ResponseFormat::JsonObject),
        };

        let json = serde_json::to_string_pretty(&full_body).unwrap();
        println!("完整 ChatBody 序列化结果:");
        println!("{}\n", json);

        // 验证所有字段都存在
        assert!(json.contains("\"request_id\": \"req_123456\""));
        assert!(json.contains("\"thinking\": \"Let me think about this...\""));
        assert!(json.contains("\"do_sample\": true"));
        assert!(json.contains("\"stream\": false"));
        assert!(json.contains("\"temperature\": 0.7"));
        assert!(json.contains("\"top_p\": 0.9"));
        assert!(json.contains("\"max_tokens\": 1000"));
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"user_id\": \"user_789\""));
        assert!(json.contains("\"stop\""));
        assert!(json.contains("\"response_format\""));
    }

    #[test]
    fn test_chatbody_with_tool_calls() {
        let model = TestModel::new("gpt-4");

        let tool_call = ToolCall::new_function("call_123", FunctionParams::new("get_weather", r#"{"location": "Beijing"}"#));
        let messages_with_tools = vec![
            TextMessage::system("You are a helpful assistant with access to weather information."),
            TextMessage::user("What's the weather like in Beijing?"),
            TextMessage::assistant_with_tools(
                Some("I'll check the weather in Beijing for you.".to_string()),
                vec![tool_call]
            ),
            TextMessage::tool_with_id("The weather in Beijing is sunny, 25°C", "call_123"),
            TextMessage::assistant("Based on the weather data, it's currently sunny in Beijing with a temperature of 25°C. It's a beautiful day!")
        ];

        let tools_body = ChatBody {
            model: model.clone(),
            messages: messages_with_tools,
            request_id: Some("req_tools_123".to_string()),
            thinking: None,
            do_sample: Some(false),
            stream: Some(true),
            temperature: Some(0.3),
            top_p: Some(0.8),
            max_tokens: Some(2000),
            tools: Some(Tools::FunctionCall {
                function_call: vec![
                    FunctionCall::new(
                        "get_weather",
                        "Get current weather information for a location",
                        r#"{"type": "object", "properties": {"location": {"type": "string", "description": "The city name"}}}"#
                    )
                ]
            }),
            user_id: Some("user_weather".to_string()),
            stop: None,
            response_format: Some(ResponseFormat::Text),
        };

        let json = serde_json::to_string_pretty(&tools_body).unwrap();
        println!("ChatBody 带工具调用消息序列化结果:");
        println!("{}\n", json);

        // 验证工具调用相关字段
        assert!(json.contains("\"tool_calls\""));
        assert!(json.contains("\"tool_call_id\""));
        assert!(json.contains("\"role\": \"tool\""));
    }

    #[test]
    fn test_chatbody_with_thinking() {
        let model = TestModel::new("gpt-4");

        let thinking_body = ChatBody {
            model: model.clone(),
            messages: vec![
                TextMessage::user("Solve this complex math problem: 2x + 5 = 15")
            ],
            request_id: Some("req_thinking".to_string()),
            thinking: None,
            do_sample: None,
            stream: None,
            temperature: Some(0.1),
            top_p: None,
            max_tokens: Some(500),
            tools: None,
            user_id: Some("user_math".to_string()),
            stop: None,
            response_format: None,
        }.with_thinking("I need to solve for x: 2x + 5 = 15, so 2x = 10, therefore x = 5");

        let json = serde_json::to_string_pretty(&thinking_body).unwrap();
        println!("ChatBody 带 thinking 序列化结果:");
        println!("{}\n", json);

        // 验证 thinking 字段
        assert!(json.contains("\"thinking\""));
        assert!(json.contains("I need to solve for x"));
    }
}