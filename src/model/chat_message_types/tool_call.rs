//! Assistant tool-call messages and their function arguments.

use serde::{Deserialize, Serialize};

/// Tool invocation emitted by an assistant message.
///
/// Function parameters are required only when [`Self::kind`] is
/// [`ToolCallType::Function`].
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub(super) id: String,
    pub(super) type_: ToolCallType,
    pub(super) function: Option<FunctionParams>,
}

impl Serialize for ToolCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::{Error as _, SerializeStruct};

        let mut state = serializer.serialize_struct("ToolCall", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", &self.type_)?;
        if self.type_ == ToolCallType::Function {
            let function = self.function.as_ref().ok_or_else(|| {
                S::Error::custom("function field is required when type is 'function'")
            })?;
            state.serialize_field("function", function)?;
        } else if let Some(function) = self.function.as_ref() {
            state.serialize_field("function", function)?;
        }
        state.end()
    }
}

/// Kind of tool invocation emitted by an assistant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallType {
    /// A function call with custom parameters.
    Function,
    /// A web-search operation.
    WebSearch,
    /// A retrieval-system access.
    Retrieval,
}

/// Function name and JSON-encoded argument string for a tool call.
///
/// The SDK preserves `arguments` exactly as supplied. Callers remain
/// responsible for validating that string against the registered schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParams {
    pub(super) name: String,
    pub(super) arguments: String,
}

impl ToolCall {
    /// Borrow the provider-issued tool-call identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the tool-call kind.
    pub fn kind(&self) -> ToolCallType {
        self.type_
    }

    /// Borrow function parameters when this is a function call.
    pub fn function(&self) -> Option<&FunctionParams> {
        self.function.as_ref()
    }

    /// Create a function invocation with its provider-issued identifier.
    pub fn new_function(id: impl Into<String>, function: FunctionParams) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Function,
            function: Some(function),
        }
    }

    /// Create a web-search invocation with its provider-issued identifier.
    pub fn new_web_search(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::WebSearch,
            function: None,
        }
    }

    /// Create a retrieval invocation with its provider-issued identifier.
    pub fn new_retrieval(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            type_: ToolCallType::Retrieval,
            function: None,
        }
    }
}

impl FunctionParams {
    /// Borrow the registered function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the unparsed JSON argument string.
    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    /// Create function parameters without parsing or normalizing arguments.
    pub fn new(name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}
