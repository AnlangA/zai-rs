//! Core traits and concrete types for defining executable tools.

use std::{borrow::Cow, collections::HashMap};

#[cfg(feature = "toolkits")]
use std::sync::{Arc, LazyLock};

use crate::toolkits::error::{ToolResult, error_context};
use async_trait::async_trait;
use serde::Serialize;

/// Compiled JSON-Schema validator used for tool-argument validation.
///
#[cfg(feature = "toolkits")]
type CompiledSchema = Arc<jsonschema::Validator>;

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
        self.metadata().name()
    }
}

/// Global schema cache for compiled JSON schemas.
#[cfg(feature = "toolkits")]
static SCHEMA_CACHE: LazyLock<std::sync::RwLock<HashMap<String, Arc<jsonschema::Validator>>>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Maximum number of compiled schemas to cache.
#[cfg(feature = "toolkits")]
const SCHEMA_CACHE_MAX_SIZE: usize = 256;

/// Metadata used to identify, describe, and categorize a tool.
#[derive(Debug, Clone, Serialize)]
pub struct ToolMetadata {
    /// Tool name (must be unique)
    name: Cow<'static, str>,

    /// Tool description
    description: Cow<'static, str>,

    /// Tool version
    version: Cow<'static, str>,

    /// Tool author
    author: Option<Cow<'static, str>>,

    /// Tool tags for categorization
    tags: Vec<Cow<'static, str>>,

    /// Whether the tool is enabled
    enabled: bool,

    /// Additional metadata
    metadata: HashMap<Cow<'static, str>, serde_json::Value>,
}

impl ToolMetadata {
    /// Create new metadata with validation
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> ToolResult<Self> {
        let name = name.into();
        let description = description.into();

        validate_tool_name(&name)?;

        Ok(Self {
            name: Cow::Owned(name),
            description: Cow::Owned(description),
            version: Cow::Borrowed("1.0.0"),
            author: None,
            tags: Vec::new(),
            enabled: true,
            metadata: HashMap::new(),
        })
    }

    /// Validated tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Human-readable description presented to the model.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Tool implementation version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Optional tool author.
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Category tags attached to the tool.
    pub fn tags(&self) -> &[Cow<'static, str>] {
        &self.tags
    }

    /// Whether the executor may run and export this tool.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Additional application-defined metadata.
    pub fn additional_metadata(&self) -> &HashMap<Cow<'static, str>, serde_json::Value> {
        &self.metadata
    }

    /// Set the tool version.
    pub fn with_version(mut self, version: impl Into<Cow<'static, str>>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the tool author.
    pub fn with_author(mut self, author: impl Into<Cow<'static, str>>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the tool's category tags.
    pub fn with_tags<T: Into<Cow<'static, str>>>(
        mut self,
        tags: impl IntoIterator<Item = T>,
    ) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Enable or disable the tool.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Attach an arbitrary key/value metadata entry.
    pub fn with_metadata(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: serde_json::Value,
    ) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

pub(crate) fn validate_tool_name(name: &str) -> ToolResult<()> {
    if name.trim().is_empty() {
        return Err(error_context().invalid_parameters("Tool name cannot be empty"));
    }
    if name.len() > 64 {
        return Err(error_context().invalid_parameters("Tool name cannot exceed 64 characters"));
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(error_context().invalid_parameters(
            "Tool name must contain only ASCII letters, digits, underscores, and hyphens",
        ));
    }
    Ok(())
}

/// Helper functions for type conversions (avoiding orphan rule issues)
pub mod conversions {
    use crate::toolkits::error::{ToolResult, error_context};

    /// Convert a value to JSON
    pub fn to_json<T: serde::Serialize>(value: T) -> ToolResult<serde_json::Value> {
        serde_json::to_value(value).map_err(|e| error_context().serialization_error(e))
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
            serde_json::Value::Number(n) => n
                .as_i64()
                .and_then(|i| i.try_into().ok())
                .ok_or_else(|| error_context().invalid_parameters("Expected i32 value")),
            _ => Err(error_context().invalid_parameters("Expected number value")),
        }
    }

    /// Extract f64 from JSON value
    pub fn from_json_f64(value: serde_json::Value) -> ToolResult<f64> {
        match value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .ok_or_else(|| error_context().invalid_parameters("Expected f64 value")),
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

// -----------------------------
// Single-struct dynamic FunctionTool
// -----------------------------

/// Shared, type-erased asynchronous handler used by registry-based tool
/// loading.
pub type ToolHandler = std::sync::Arc<
    dyn Fn(
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>,
        > + Send
        + Sync,
>;

/// A single-struct tool that carries metadata, JSON schema, and an async
/// handler
pub struct FunctionTool {
    metadata: ToolMetadata,
    input_schema: serde_json::Value,
    #[cfg(feature = "toolkits")]
    compiled_schema: CompiledSchema,
    handler: ToolHandler,
}

impl Clone for FunctionTool {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            input_schema: self.input_schema.clone(),
            #[cfg(feature = "toolkits")]
            compiled_schema: Arc::clone(&self.compiled_schema),
            handler: self.handler.clone(),
        }
    }
}

impl FunctionTool {
    /// Start building a [`FunctionTool`] with the given name and description.
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> FunctionToolBuilder {
        FunctionToolBuilder::new(name, description)
    }
    /// Convenience: build a FunctionTool directly from a full JSON schema and a
    /// handler
    pub fn from_schema<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: serde_json::Value,
        f: F,
    ) -> ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResult<serde_json::Value>> + Send + 'static,
    {
        Self::builder(name, description)
            .schema(schema)
            .handler(f)
            .build()
    }
    /// Build a FunctionTool from a full JSON spec (supports two shapes):
    /// 1) {"name":..., "description":..., "parameters": {...}}
    /// 2) {"type":"function", "function": {"name":..., "description":...,
    ///    "parameters": {...}}}
    pub fn from_function_spec<F, Fut>(spec: serde_json::Value, f: F) -> ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResult<serde_json::Value>> + Send + 'static,
    {
        let (name, description, parameters) = parse_function_spec_details(&spec)?;
        let mut builder = Self::builder(name, description);
        if let Some(p) = parameters {
            builder = builder.schema(p);
        }
        builder.handler(f).build()
    }

    /// Read a JSON function spec from a file and build a FunctionTool.
    pub fn from_function_spec_file<F, Fut>(
        path: impl AsRef<std::path::Path>,
        f: F,
    ) -> ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResult<serde_json::Value>> + Send + 'static,
    {
        let content = std::fs::read_to_string(path).map_err(|e| {
            error_context().invalid_parameters(format!("Failed to read spec file: {e}"))
        })?;
        let spec: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| error_context().invalid_parameters(format!("Invalid JSON: {e}")))?;
        Self::from_function_spec(spec, f)
    }
}

#[cfg(feature = "toolkits")]
/// Compile JSON schema with caching for better performance
fn compile_schema_cached(
    schema: &serde_json::Value,
    tool_name: &str,
) -> ToolResult<Arc<jsonschema::Validator>> {
    // The canonical schema itself is the cache key. A short hash could collide
    // and reuse the wrong validator, which would change validation semantics.
    let cache_key = schema.to_string();

    // Check cache first
    {
        // Recover from a poisoned lock (a prior panic while holding it) by
        // taking the inner guard rather than panicking here.
        let cache = SCHEMA_CACHE
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(Arc::clone(cached));
        }
    }

    // Compile and cache
    let validator = jsonschema::validator_for(schema).map_err(|e| {
        error_context()
            .with_tool(tool_name)
            .schema_validation(format!("Failed to compile schema: {e}"))
    })?;

    let validator = Arc::new(validator);

    {
        let mut cache = SCHEMA_CACHE
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Evict a slice of entries if cache is full
        if cache.len() >= SCHEMA_CACHE_MAX_SIZE {
            // Remove ~10% of entries to make room. `HashMap` iteration order is
            // unspecified, so the evicted slice is arbitrary rather than the
            // oldest — acceptable for a bounded, registration-time cache.
            let remove_count = (SCHEMA_CACHE_MAX_SIZE / 10).max(1);
            let keys: Vec<String> = cache.keys().take(remove_count).cloned().collect();
            for k in keys {
                cache.remove(&k);
            }
        }
        cache.insert(cache_key, Arc::clone(&validator));
    }

    Ok(validator)
}

/// (internal) Parses the name, description, and parameters from a JSON function
/// spec.
pub(crate) fn parse_function_spec_details(
    spec: &serde_json::Value,
) -> ToolResult<(String, String, Option<serde_json::Value>)> {
    use serde_json::Value;
    let obj = match spec {
        Value::Object(map) => map,
        _ => return Err(error_context().invalid_parameters("Function spec must be a JSON object")),
    };
    // Shape 2 with outer {type:function, function:{...}}
    let (name, desc, params) = if obj.get("type").and_then(|v| v.as_str()) == Some("function") {
        let f = obj
            .get("function")
            .and_then(|v| v.as_object())
            .ok_or_else(|| error_context().invalid_parameters("Missing 'function' object"))?;
        let name = f
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| error_context().invalid_parameters("Missing function.name"))?
            .to_string();
        let desc = f
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let params = f.get("parameters").cloned();
        (name, desc, params)
    } else {
        // Shape 1 inner {name, description, parameters}
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| error_context().invalid_parameters("Missing name"))?
            .to_string();
        let desc = obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let params = obj.get("parameters").cloned();
        (name, desc, params)
    };
    Ok((name, desc, params))
}

/// Builder for FunctionTool
pub struct FunctionToolBuilder {
    metadata: ToolMetadata,
    input_schema: Option<serde_json::Value>,
    /// Schema fragments accumulated by the fluent property API.
    staged_properties: Option<serde_json::Map<String, serde_json::Value>>,
    staged_required: Vec<String>,
    handler: Option<ToolHandler>,
}

impl FunctionToolBuilder {
    /// Create a new builder for a tool with the given name and description.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        // Preserve invalid input until `build`: silently renaming an invalid
        // tool to `unknown` hides configuration errors and creates collisions.
        let metadata = ToolMetadata {
            name: Cow::Owned(name.into()),
            description: Cow::Owned(description.into()),
            version: Cow::Borrowed("1.0.0"),
            author: None,
            tags: Vec::new(),
            enabled: true,
            metadata: HashMap::new(),
        };
        Self {
            metadata,
            input_schema: None,
            staged_properties: None,
            staged_required: Vec::new(),
            handler: None,
        }
    }

    /// Provide the full input JSON schema directly.
    pub fn schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Mutate the tool's metadata (e.g. version, tags) via a closure.
    pub fn metadata(mut self, f: impl FnOnce(ToolMetadata) -> ToolMetadata) -> Self {
        self.metadata = f(self.metadata);
        self
    }

    /// Set the async handler invoked on each (validated) call.
    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolResult<serde_json::Value>> + Send + 'static,
    {
        let wrapped = move |args: serde_json::Value| -> std::pin::Pin<
            Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>,
        > { Box::pin(f(args)) };
        self.handler = Some(std::sync::Arc::new(wrapped));
        self
    }

    /// Chain API: add one property to the schema. If `schema(json!(...))` is
    /// also provided, the property will be merged into its `properties`
    /// object.
    pub fn property(mut self, name: impl Into<String>, schema: serde_json::Value) -> Self {
        let name = name.into();
        let entry = self
            .staged_properties
            .get_or_insert_with(serde_json::Map::new);
        entry.insert(name, schema);
        self
    }

    /// Chain API: mark a property as required. Will be merged with any provided
    /// schema's `required`.
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.staged_required.push(name.into());
        self
    }

    /// Finalize the tool: validates the handler is set, compiles the schema,
    /// and returns the built [`FunctionTool`].
    pub fn build(mut self) -> ToolResult<FunctionTool> {
        validate_tool_name(&self.metadata.name)?;
        let handler = self
            .handler
            .ok_or_else(|| error_context().invalid_parameters("FunctionTool handler not set"))?;
        let mut schema = self
            .input_schema
            .take()
            .unwrap_or_else(|| serde_json::json!({}));

        if let serde_json::Value::Object(ref mut obj) = schema {
            if obj
                .get("type")
                .is_some_and(|schema_type| schema_type.as_str() != Some("object"))
            {
                return Err(error_context()
                    .with_tool(self.metadata.name.clone())
                    .invalid_parameters("tool input schema type must be 'object'"));
            }
            obj.entry("type")
                .or_insert(serde_json::Value::String("object".to_string()));
            obj.entry("additionalProperties")
                .or_insert(serde_json::Value::Bool(false));

            if let Some(staged) = self.staged_properties.take() {
                let props = obj
                    .entry("properties")
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                let props = props.as_object_mut().ok_or_else(|| {
                    error_context()
                        .with_tool(self.metadata.name.clone())
                        .invalid_parameters("schema.properties must be an object")
                })?;
                for (name, property_schema) in staged {
                    props.insert(name, property_schema);
                }
            }

            if !self.staged_required.is_empty() {
                use std::collections::BTreeSet;
                let mut required = BTreeSet::new();
                if let Some(existing) = obj.get("required") {
                    let entries = existing.as_array().ok_or_else(|| {
                        error_context()
                            .with_tool(self.metadata.name.clone())
                            .invalid_parameters("schema.required must be an array of strings")
                    })?;
                    for entry in entries {
                        let name = entry.as_str().ok_or_else(|| {
                            error_context()
                                .with_tool(self.metadata.name.clone())
                                .invalid_parameters("schema.required must contain only strings")
                        })?;
                        required.insert(name.to_string());
                    }
                }
                required.extend(self.staged_required);
                obj.insert(
                    "required".to_string(),
                    serde_json::Value::Array(
                        required
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                );
            }
            obj.entry("properties")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        } else {
            return Err(error_context()
                .with_tool(self.metadata.name.clone())
                .invalid_parameters("tool input schema must be a JSON object"));
        }

        #[cfg(feature = "toolkits")]
        let compiled_schema: CompiledSchema = compile_schema_cached(&schema, &self.metadata.name)?;
        Ok(FunctionTool {
            metadata: self.metadata,
            input_schema: schema,
            #[cfg(feature = "toolkits")]
            compiled_schema,
            handler,
        })
    }
}

#[async_trait]
impl DynTool for FunctionTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute_json(&self, input: serde_json::Value) -> ToolResult<serde_json::Value> {
        // Validate the input against the compiled schema (only when enabled).
        #[cfg(feature = "toolkits")]
        if let Err(validation_error) = self.compiled_schema.validate(&input) {
            return Err(error_context()
                .with_tool(self.name())
                .invalid_parameters(format!("Input validation failed: {validation_error}")));
        }

        // If validation passes (or is disabled), execute the handler
        (self.handler)(input).await
    }

    fn input_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkits::ToolError;

    #[test]
    fn test_tool_metadata_new() {
        let metadata = ToolMetadata::new("test_tool", "A test tool").unwrap();
        assert_eq!(metadata.name(), "test_tool");
        assert_eq!(metadata.description(), "A test tool");
        assert_eq!(metadata.version(), "1.0.0");
        assert!(metadata.is_enabled());

        let hyphenated = ToolMetadata::new("test-tool", "A test tool").unwrap();
        assert_eq!(hyphenated.name(), "test-tool");
    }

    #[test]
    fn test_tool_metadata_invalid_name_empty() {
        let result = ToolMetadata::new("", "A test tool");
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidParameters { .. } => {},
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_tool_metadata_invalid_name_special_chars() {
        let result = ToolMetadata::new("test-tool!", "A test tool");
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidParameters { .. } => {},
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_tool_metadata_builder() {
        let metadata = ToolMetadata::new("test_tool", "A test tool")
            .unwrap()
            .with_version("2.0.0")
            .with_author("Test Author")
            .with_tags(["tag1", "tag2"])
            .with_enabled(false);

        assert_eq!(metadata.version(), "2.0.0");
        assert_eq!(metadata.author(), Some("Test Author"));
        assert_eq!(metadata.tags().len(), 2);
        assert!(!metadata.is_enabled());
    }

    #[test]
    fn test_conversions_to_json() {
        let value = conversions::to_json(42).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_conversions_from_json_string() {
        let value = serde_json::Value::String("hello".to_string());
        let result = conversions::from_json_string(value).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_conversions_from_json_string_invalid() {
        let value = serde_json::Value::Number(42.into());
        let result = conversions::from_json_string(value);
        assert!(result.is_err());
    }

    #[test]
    fn test_conversions_from_json_i32() {
        let value = serde_json::Value::Number(42.into());
        let result = conversions::from_json_i32(value).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_conversions_from_json_f64() {
        let value = serde_json::json!(3.5);
        let result = conversions::from_json_f64(value).unwrap();
        assert_eq!(result, 3.5);
    }

    #[test]
    fn test_conversions_from_json_bool() {
        let value = serde_json::Value::Bool(true);
        let result = conversions::from_json_bool(value).unwrap();
        assert!(result);
    }

    #[test]
    fn test_function_tool_builder() {
        let tool = FunctionTool::builder("test_tool", "A test tool")
            .property("param1", serde_json::json!({"type": "string"}))
            .property("param2", serde_json::json!({"type": "number"}))
            .required("param1")
            .handler(|_args| async move { Ok(serde_json::json!({"result": "ok"})) })
            .build();

        assert!(tool.is_ok());
        let tool = tool.unwrap();
        assert_eq!(tool.name(), "test_tool");
    }

    #[test]
    fn test_function_tool_clone() {
        let tool1 = FunctionTool::builder("test_tool", "A test tool")
            .property("param1", serde_json::json!({"type": "string"}))
            .required("param1")
            .handler(|_args| async move { Ok(serde_json::json!({"result": "ok"})) })
            .build()
            .unwrap();

        let tool2 = tool1.clone();
        assert_eq!(tool1.name(), tool2.name());
        assert_eq!(tool1.input_schema(), tool2.input_schema());
    }

    #[test]
    fn function_tool_rejects_non_object_input_schemas() {
        for schema in [
            serde_json::json!(true),
            serde_json::json!({"type": "string"}),
            serde_json::json!([{"type": "object"}]),
        ] {
            let result = FunctionTool::builder("invalid_schema", "test fixture")
                .schema(schema)
                .handler(|input| async move { Ok(input) })
                .build();
            assert!(matches!(result, Err(ToolError::InvalidParameters { .. })));
        }
    }

    #[test]
    fn test_parse_function_spec_shape1() {
        let spec = serde_json::json!({
            "name": "test_tool",
            "description": "A test tool",
            "parameters": {
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }
        });

        let (name, description, parameters) = parse_function_spec_details(&spec).unwrap();
        assert_eq!(name, "test_tool");
        assert_eq!(description, "A test tool");
        assert!(parameters.is_some());
    }

    #[test]
    fn test_parse_function_spec_shape2() {
        let spec = serde_json::json!({
            "type": "function",
            "function": {
                "name": "test_tool",
                "description": "A test tool",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "param1": {"type": "string"}
                    }
                }
            }
        });

        let (name, description, parameters) = parse_function_spec_details(&spec).unwrap();
        assert_eq!(name, "test_tool");
        assert_eq!(description, "A test tool");
        assert!(parameters.is_some());
    }

    #[test]
    fn test_parse_function_spec_invalid() {
        let spec = serde_json::Value::String("invalid".to_string());
        let result = parse_function_spec_details(&spec);
        assert!(result.is_err());
    }
}
