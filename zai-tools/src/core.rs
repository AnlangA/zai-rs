//! Core traits and types with enhanced type safety

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::any::TypeId;
use std::collections::HashMap;

use crate::error::{ToolResult, error_context};

/// Trait for tool input parameters
pub trait ToolInput: DeserializeOwned + Send + Sync + 'static {
    /// Validate the input parameters
    fn validate(&self) -> ToolResult<()> {
        Ok(())
    }

    /// Get the JSON schema for this input type
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "description": "Tool input parameters"
        })
    }

    /// Get the type name for debugging
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Convert from JSON value with validation
    fn from_json(value: serde_json::Value) -> ToolResult<Self> {
        let input: Self = serde_json::from_value(value)
            .map_err(|e| error_context().serialization_error(e))?;
        input.validate()?;
        Ok(input)
    }
}

/// Trait for tool output results
pub trait ToolOutput: Serialize + Send + Sync + 'static {
    /// Convert to JSON value
    fn to_json(&self) -> ToolResult<serde_json::Value> {
        serde_json::to_value(self)
            .map_err(|e| error_context().serialization_error(e))
    }

    /// Get the type name for debugging
    fn type_name() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Convert to pretty JSON string
    fn to_json_pretty(&self) -> ToolResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| error_context().serialization_error(e))
    }
}

/// Enhanced Tool trait with type safety
#[async_trait]
pub trait Tool<I: ToolInput, O: ToolOutput>: Send + Sync {
    /// Get the tool's metadata
    fn metadata(&self) -> &ToolMetadata;
    
    /// Execute the tool with typed parameters
    async fn execute(&self, input: I) -> ToolResult<O>;
    
    /// Get the input type ID for runtime type checking
    fn input_type_id(&self) -> TypeId {
        TypeId::of::<I>()
    }
    
    /// Get the output type ID for runtime type checking
    fn output_type_id(&self) -> TypeId {
        TypeId::of::<O>()
    }
}

/// Type-erased tool trait for dynamic dispatch
#[async_trait]
pub trait DynTool: Send + Sync {
    /// Get the tool's metadata
    fn metadata(&self) -> &ToolMetadata;
    
    /// Execute with JSON input/output
    async fn execute_json(&self, input: serde_json::Value) -> ToolResult<serde_json::Value>;
    
    /// Get input schema
    fn input_schema(&self) -> serde_json::Value;
    
    /// Get the tool name
    fn name(&self) -> &str {
        &self.metadata().name
    }
    
    /// Clone the tool as a boxed trait object
    fn clone_box(&self) -> Box<dyn DynTool>;
}

/// Wrapper to convert typed tools to dynamic tools
pub struct ToolWrapper<T, I, O>
where
    T: Tool<I, O>,
    I: ToolInput,
    O: ToolOutput,
{
    tool: T,
    _phantom: std::marker::PhantomData<(I, O)>,
}

impl<T, I, O> ToolWrapper<T, I, O>
where
    T: Tool<I, O>,
    I: ToolInput,
    O: ToolOutput,
{
    pub fn new(tool: T) -> Self {
        Self {
            tool,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, I, O> DynTool for ToolWrapper<T, I, O>
where
    T: Tool<I, O> + Clone + 'static,
    I: ToolInput,
    O: ToolOutput,
{
    fn metadata(&self) -> &ToolMetadata {
        self.tool.metadata()
    }
    
    async fn execute_json(&self, input: serde_json::Value) -> ToolResult<serde_json::Value> {
        let typed_input: I = serde_json::from_value(input)
            .map_err(|e| error_context()
                .with_tool(self.name())
                .serialization_error(e))?;
        
        typed_input.validate()
            .map_err(|e| error_context()
                .with_tool(self.name())
                .invalid_parameters(format!("Validation failed: {}", e)))?;
        
        let result = self.tool.execute(typed_input).await?;
        result.to_json()
    }
    
    fn input_schema(&self) -> serde_json::Value {
        I::schema()
    }
    
    fn clone_box(&self) -> Box<dyn DynTool> {
        Box::new(ToolWrapper::new(self.tool.clone()))
    }
}

/// Enhanced tool metadata with better type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (must be unique)
    pub name: String,
    
    /// Tool description
    pub description: String,
    
    /// Tool version
    pub version: String,
    
    /// Tool author
    pub author: Option<String>,
    
    /// Tool tags for categorization
    pub tags: Vec<String>,
    
    /// Whether the tool is enabled
    pub enabled: bool,
    
    /// Input type name for debugging
    pub input_type: String,
    
    /// Output type name for debugging
    pub output_type: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolMetadata {
    /// Create new metadata with type information
    pub fn new<I: ToolInput, O: ToolOutput>(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            enabled: true,
            input_type: std::any::type_name::<I>().to_string(),
            output_type: std::any::type_name::<O>().to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// Builder pattern methods
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
    
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
    
    pub fn tags<T: Into<String>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
    
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Convenience implementations for common types
impl ToolInput for () {}
impl ToolOutput for () {}

impl ToolInput for String {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "string",
            "description": "String input"
        })
    }
}

impl ToolOutput for String {}

impl ToolInput for serde_json::Value {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "description": "JSON object input"
        })
    }
}

impl ToolOutput for serde_json::Value {}

/// Macro to implement ToolInput for simple types
macro_rules! impl_tool_input_for_primitives {
    ($($t:ty => $json_type:expr),*) => {
        $(
            impl ToolInput for $t {
                fn schema() -> serde_json::Value {
                    serde_json::json!({
                        "type": $json_type,
                        "description": concat!("Input of type ", stringify!($t))
                    })
                }
            }
            
            impl ToolOutput for $t {}
        )*
    };
}

impl_tool_input_for_primitives! {
    i32 => "integer",
    i64 => "integer",
    u32 => "integer",
    u64 => "integer",
    usize => "integer",
    isize => "integer",
    f32 => "number",
    f64 => "number",
    bool => "boolean"
}

// Additional implementations for common types
impl<T> ToolInput for Vec<T> where T: serde::de::DeserializeOwned + Send + Sync + 'static {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "description": "Array input"
        })
    }
}

impl<T> ToolOutput for Vec<T> where T: serde::Serialize + Send + Sync + 'static {}

impl<T> ToolInput for Option<T> where T: ToolInput {
    fn schema() -> serde_json::Value {
        let mut schema = T::schema();
        if let serde_json::Value::Object(ref mut obj) = schema {
            obj.insert("nullable".to_string(), serde_json::Value::Bool(true));
        }
        schema
    }
}

impl<T> ToolOutput for Option<T> where T: ToolOutput {}

impl<A, B> ToolInput for (A, B)
where
    A: serde::de::DeserializeOwned + Send + Sync + 'static,
    B: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": [
                {"type": "object"},
                {"type": "object"}
            ],
            "minItems": 2,
            "maxItems": 2,
            "description": "Tuple input"
        })
    }
}

impl<A, B> ToolOutput for (A, B)
where
    A: serde::Serialize + Send + Sync + 'static,
    B: serde::Serialize + Send + Sync + 'static,
{}

impl<A, B, C> ToolInput for (A, B, C)
where
    A: serde::de::DeserializeOwned + Send + Sync + 'static,
    B: serde::de::DeserializeOwned + Send + Sync + 'static,
    C: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": [
                {"type": "object"},
                {"type": "object"},
                {"type": "object"}
            ],
            "minItems": 3,
            "maxItems": 3,
            "description": "Triple input"
        })
    }
}

impl<A, B, C> ToolOutput for (A, B, C)
where
    A: serde::Serialize + Send + Sync + 'static,
    B: serde::Serialize + Send + Sync + 'static,
    C: serde::Serialize + Send + Sync + 'static,
{}

/// Helper trait for converting tools to dynamic tools
pub trait IntoDynTool<I: ToolInput, O: ToolOutput> {
    fn into_dyn_tool(self) -> Box<dyn DynTool>;
}

impl<T, I, O> IntoDynTool<I, O> for T
where
    T: Tool<I, O> + Clone + 'static,
    I: ToolInput,
    O: ToolOutput,
{
    fn into_dyn_tool(self) -> Box<dyn DynTool> {
        Box::new(ToolWrapper::new(self))
    }
}

/// Helper functions for type conversions (avoiding orphan rule issues)
pub mod conversions {
    use crate::error::{ToolResult, error_context};

    /// Convert a value to JSON
    pub fn to_json<T: serde::Serialize>(value: T) -> ToolResult<serde_json::Value> {
        serde_json::to_value(value)
            .map_err(|e| error_context().serialization_error(e))
    }

    /// Extract string from JSON value
    pub fn from_json_string(value: serde_json::Value) -> ToolResult<String> {
        match value {
            serde_json::Value::String(s) => Ok(s),
            _ => Err(error_context().invalid_parameters("Expected string value")),
        }
    }

    /// Extract i32 from JSON value
    pub fn from_json_i32(value: serde_json::Value) -> ToolResult<i32> {
        match value {
            serde_json::Value::Number(n) => {
                n.as_i64()
                    .and_then(|i| i.try_into().ok())
                    .ok_or_else(|| error_context().invalid_parameters("Expected i32 value"))
            }
            _ => Err(error_context().invalid_parameters("Expected number value")),
        }
    }

    /// Extract f64 from JSON value
    pub fn from_json_f64(value: serde_json::Value) -> ToolResult<f64> {
        match value {
            serde_json::Value::Number(n) => {
                n.as_f64()
                    .ok_or_else(|| error_context().invalid_parameters("Expected f64 value"))
            }
            _ => Err(error_context().invalid_parameters("Expected number value")),
        }
    }

    /// Extract bool from JSON value
    pub fn from_json_bool(value: serde_json::Value) -> ToolResult<bool> {
        match value {
            serde_json::Value::Bool(b) => Ok(b),
            _ => Err(error_context().invalid_parameters("Expected boolean value")),
        }
    }
}
