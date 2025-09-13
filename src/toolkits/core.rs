//! Core traits and types with enhanced type safety

use async_trait::async_trait;
use jsonschema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::borrow::Cow;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::toolkits::error::{ToolResult, error_context};

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

/// Global schema cache for compiled JSON schemas
static SCHEMA_CACHE: Lazy<RwLock<HashMap<u64, Arc<jsonschema::Validator>>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Enhanced tool metadata with better type information and memory optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (must be unique)
    pub name: Cow<'static, str>,

    /// Tool description
    pub description: Cow<'static, str>,

    /// Tool version
    pub version: Cow<'static, str>,

    /// Tool author
    pub author: Option<Cow<'static, str>>,

    /// Tool tags for categorization
    pub tags: Vec<Cow<'static, str>>,

    /// Whether the tool is enabled
    pub enabled: bool,

    /// Additional metadata
    pub metadata: HashMap<Cow<'static, str>, serde_json::Value>,
}

impl ToolMetadata {
    /// Create new metadata with validation
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> ToolResult<Self> {
        let name = name.into();
        let description = description.into();
        
        // Validate tool name
        if name.trim().is_empty() {
            return Err(error_context().invalid_parameters("Tool name cannot be empty"));
        }
        if name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
            return Err(error_context().invalid_parameters("Tool name must be alphanumeric with underscores only"));
        }
        
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

    /// Builder pattern methods
    pub fn version(mut self, version: impl Into<Cow<'static, str>>) -> Self {
        self.version = version.into();
        self
    }

    pub fn author(mut self, author: impl Into<Cow<'static, str>>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn tags<T: Into<Cow<'static, str>>>(mut self, tags: impl IntoIterator<Item = T>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<Cow<'static, str>>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
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

/// A single-struct tool that carries metadata, JSON schema, and an async handler
pub struct FunctionTool {
    metadata: ToolMetadata,
    input_schema: serde_json::Value,
    compiled_schema: Arc<jsonschema::Validator>,
    handler: std::sync::Arc<
        dyn Fn(
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = crate::toolkits::error::ToolResult<serde_json::Value>,
                        > + Send,
                >,
            > + Send
            + Sync,
    >,
}

impl Clone for FunctionTool {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            input_schema: self.input_schema.clone(),
            compiled_schema: Arc::clone(&self.compiled_schema),
            handler: self.handler.clone(),
        }
    }
}

impl FunctionTool {
    pub fn builder(name: impl Into<String>, description: impl Into<String>) -> FunctionToolBuilder {
        FunctionToolBuilder::new(name, description)
    }
    /// Convenience: build a FunctionTool directly from a full JSON schema and a handler
    pub fn from_schema<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: serde_json::Value,
        f: F,
    ) -> crate::toolkits::error::ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::toolkits::error::ToolResult<serde_json::Value>>
            + Send
            + 'static,
    {
        Self::builder(name, description)
            .schema(schema)
            .handler(f)
            .build()
    }
    /// Build a FunctionTool from a full JSON spec (supports two shapes):
    /// 1) {"name":..., "description":..., "parameters": {...}}
    /// 2) {"type":"function", "function": {"name":..., "description":..., "parameters": {...}}}
    pub fn from_function_spec<F, Fut>(
        spec: serde_json::Value,
        f: F,
    ) -> crate::toolkits::error::ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::toolkits::error::ToolResult<serde_json::Value>>
            + Send
            + 'static,
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
    ) -> crate::toolkits::error::ToolResult<FunctionTool>
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::toolkits::error::ToolResult<serde_json::Value>>
            + Send
            + 'static,
    {
        let content = std::fs::read_to_string(path).map_err(|e| {
            error_context().invalid_parameters(format!("Failed to read spec file: {}", e))
        })?;
        let spec: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| error_context().invalid_parameters(format!("Invalid JSON: {}", e)))?;
        Self::from_function_spec(spec, f)
    }
}

/// Compile JSON schema with caching for better performance
fn compile_schema_cached(schema: &serde_json::Value) -> ToolResult<Arc<jsonschema::Validator>> {
    let mut hasher = DefaultHasher::new();
    schema.to_string().hash(&mut hasher);
    let hash = hasher.finish();
    
    // Check cache first
    {
        let cache = SCHEMA_CACHE.read();
        if let Some(cached) = cache.get(&hash) {
            return Ok(Arc::clone(cached));
        }
    }
    
    // Compile and cache
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| error_context().schema_validation(format!("Failed to compile schema: {}", e)))?;
    
    let validator = Arc::new(validator);
    
    {
        let mut cache = SCHEMA_CACHE.write();
        cache.insert(hash, Arc::clone(&validator));
    }
    
    Ok(validator)
}

/// (internal) Parses the name, description, and parameters from a JSON function spec.
pub(crate) fn parse_function_spec_details(
    spec: &serde_json::Value,
) -> crate::toolkits::error::ToolResult<(String, String, Option<serde_json::Value>)> {
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
    // Optional staged schema pieces for convenience building when schema() is omitted or for merging
    staged_properties: Option<serde_json::Map<String, serde_json::Value>>,
    staged_required: Vec<String>,
    handler: Option<
        std::sync::Arc<
            dyn Fn(
                    serde_json::Value,
                ) -> std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = crate::toolkits::error::ToolResult<serde_json::Value>,
                            > + Send,
                    >,
                > + Send
                + Sync,
        >,
    >,
}

impl FunctionToolBuilder {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            metadata: ToolMetadata::new(name, description).unwrap_or_else(|_| {
                ToolMetadata {
                    name: Cow::Borrowed("unknown"),
                    description: Cow::Borrowed("unknown"),
                    version: Cow::Borrowed("1.0.0"),
                    author: None,
                    tags: Vec::new(),
                    enabled: true,
                    metadata: HashMap::new(),
                }
            }),
            input_schema: None,
            staged_properties: None,
            staged_required: Vec::new(),
            handler: None,
        }
    }

    pub fn schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    pub fn metadata(mut self, f: impl FnOnce(ToolMetadata) -> ToolMetadata) -> Self {
        self.metadata = f(self.metadata);
        self
    }

    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::toolkits::error::ToolResult<serde_json::Value>>
            + Send
            + 'static,
    {
        let wrapped = move |args: serde_json::Value| -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = crate::toolkits::error::ToolResult<serde_json::Value>,
                    > + Send,
            >,
        > { Box::pin(f(args)) };
        self.handler = Some(std::sync::Arc::new(wrapped));
        self
    }

    /// Chain API: add one property to the schema. If `schema(json!(...))` is also provided,
    /// the property will be merged into its `properties` object.
    pub fn property(mut self, name: impl Into<String>, schema: serde_json::Value) -> Self {
        let name = name.into();
        let entry = self
            .staged_properties
            .get_or_insert_with(serde_json::Map::new);
        entry.insert(name, schema);
        self
    }

    /// Chain API: mark a property as required. Will be merged with any provided schema's `required`.
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.staged_required.push(name.into());
        self
    }

    pub fn build(mut self) -> crate::toolkits::error::ToolResult<FunctionTool> {
        let handler = self
            .handler
            .ok_or_else(|| error_context().invalid_parameters("FunctionTool handler not set"))?;
        // Start with provided schema or an empty object to fill
        let mut schema = self
            .input_schema
            .take()
            .unwrap_or_else(|| serde_json::json!({}));

        // If schema is an object, we can augment it; otherwise leave it as-is
        if let serde_json::Value::Object(ref mut obj) = schema {
            // Ensure required base shape
            obj.entry("type")
                .or_insert(serde_json::Value::String("object".to_string()));
            obj.entry("additionalProperties")
                .or_insert(serde_json::Value::Bool(false));

            // Merge staged properties (if any)
            if let Some(staged) = self.staged_properties.take() {
                let props = obj
                    .entry("properties")
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                if let serde_json::Value::Object(props_obj) = props {
                    for (k, v) in staged {
                        props_obj.insert(k, v);
                    }
                }
            }
            // Merge staged required (if any), de-duplicated
            if !self.staged_required.is_empty() {
                use std::collections::BTreeSet;
                let mut set: BTreeSet<String> = obj
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                for r in self.staged_required.into_iter() {
                    set.insert(r);
                }
                obj.insert(
                    "required".to_string(),
                    serde_json::Value::Array(
                        set.into_iter().map(serde_json::Value::String).collect(),
                    ),
                );
            }
        } else {
            // If schema is not an object and also not provided, enforce default
            // But since we only hit here when schema is not an object (provided by user), we leave it.
        }

        // If user provided nothing and schema is empty object, ensure defaults
        if let serde_json::Value::Object(ref mut obj) = schema {
            obj.entry("type")
                .or_insert(serde_json::Value::String("object".to_string()));
            obj.entry("additionalProperties")
                .or_insert(serde_json::Value::Bool(false));
            // Ensure properties exists when we staged some but merging didn't set (edge case)
            if obj.get("properties").is_none() {
                obj.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(serde_json::Map::new()),
                );
            }
        }

        let compiled_schema = compile_schema_cached(&schema).map_err(|e| {
            error_context()
                .with_tool(self.metadata.name.clone())
                .schema_validation(format!("Failed to compile schema: {}", e))
        })?;

        Ok(FunctionTool {
            metadata: self.metadata,
            input_schema: schema,
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
        // Validate the input against the compiled schema
        if let Err(validation_error) = self.compiled_schema.validate(&input) {
            return Err(error_context()
                .with_tool(self.name())
                .invalid_parameters(format!("Input validation failed: {}", validation_error)));
        }

        // If validation passes, execute the handler
        (self.handler)(input).await
    }

    fn input_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }

    fn clone_box(&self) -> Box<dyn DynTool> {
        Box::new(self.clone())
    }
}
