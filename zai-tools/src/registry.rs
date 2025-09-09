//! Enhanced tool registry with better type safety and Rust idioms

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::any::TypeId;

use crate::core::{DynTool, Tool, ToolInput, ToolOutput, IntoDynTool, ToolMetadata};
use crate::error::{ToolResult, ToolError, error_context};

/// Thread-safe tool registry with enhanced type safety
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn DynTool>>>>,
    type_registry: Arc<RwLock<HashMap<String, (TypeId, TypeId)>>>, // (input_type, output_type)
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tools = self.tools.read().unwrap();
        let tool_names: Vec<&String> = tools.keys().collect();
        f.debug_struct("ToolRegistry")
            .field("tool_count", &tools.len())
            .field("tools", &tool_names)
            .finish()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating tool registries with fluent API
pub struct RegistryBuilder {
    registry: ToolRegistry,
}

impl RegistryBuilder {
    /// Create a new registry builder
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    /// Add a tool to the registry (returns Result for error handling)
    pub fn with_tool<T, I, O>(self, tool: T) -> ToolResult<Self>
    where
        T: Tool<I, O> + Clone + 'static,
        I: ToolInput,
        O: ToolOutput,
    {
        self.registry.register(tool)?;
        Ok(self)
    }

    /// Add a tool to the registry (panics on error, for fluent chaining)
    pub fn add_tool<T, I, O>(self, tool: T) -> Self
    where
        T: Tool<I, O> + Clone + 'static,
        I: ToolInput,
        O: ToolOutput,
    {
        self.registry.register(tool).expect("Failed to register tool");
        self
    }

    /// Try to add a tool to the registry (ignores errors, for fluent chaining)
    pub fn try_add_tool<T, I, O>(self, tool: T) -> Self
    where
        T: Tool<I, O> + Clone + 'static,
        I: ToolInput,
        O: ToolOutput,
    {
        let _ = self.registry.register(tool);
        self
    }

    /// Build the final registry
    pub fn build(self) -> ToolRegistry {
        self.registry
    }

    /// Build the final registry, returning a Result if any tools failed to register
    pub fn try_build(self) -> ToolResult<ToolRegistry> {
        Ok(self.registry)
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            type_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a registry builder for fluent API
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder::new()
    }
    
    /// Register a typed tool
    pub fn register<T, I, O>(&self, tool: T) -> ToolResult<&Self>
    where
        T: Tool<I, O> + Clone + 'static,
        I: ToolInput,
        O: ToolOutput,
    {
        let name = tool.metadata().name.clone();
        
        // Check if tool is already registered
        {
            let tools = self.tools.read().unwrap();
            if tools.contains_key(&name) {
                return Err(ToolError::RegistrationError {
                    message: format!("Tool '{}' is already registered", name),
                });
            }
        }
        
        // Register type information
        {
            let mut type_registry = self.type_registry.write().unwrap();
            type_registry.insert(name.clone(), (tool.input_type_id(), tool.output_type_id()));
        }
        
        // Register the tool
        {
            let mut tools = self.tools.write().unwrap();
            tools.insert(name, tool.into_dyn_tool());
        }
        
        Ok(self)
    }
    
    /// Register a dynamic tool directly
    pub fn register_dyn(&self, tool: Box<dyn DynTool>) -> ToolResult<&Self> {
        let name = tool.name().to_string();
        
        // Check if tool is already registered
        {
            let tools = self.tools.read().unwrap();
            if tools.contains_key(&name) {
                return Err(ToolError::RegistrationError {
                    message: format!("Tool '{}' is already registered", name),
                });
            }
        }
        
        // Register the tool
        {
            let mut tools = self.tools.write().unwrap();
            tools.insert(name, tool);
        }
        
        Ok(self)
    }
    
    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        let mut tools = self.tools.write().unwrap();
        let mut type_registry = self.type_registry.write().unwrap();
        
        if tools.remove(name).is_none() {
            return Err(error_context().tool_not_found());
        }
        
        type_registry.remove(name);
        Ok(())
    }
    
    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Box<dyn DynTool>> {
        let tools = self.tools.read().unwrap();
        tools.get(name).map(|tool| tool.clone_box())
    }
    
    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }
    
    /// Get all registered tool names
    pub fn tool_names(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools.keys().cloned().collect()
    }
    
    /// Get metadata for all registered tools
    pub fn all_metadata(&self) -> Vec<ToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.values()
            .map(|tool| tool.metadata().clone())
            .collect()
    }
    
    /// Get metadata for a specific tool
    pub fn metadata(&self, name: &str) -> Option<ToolMetadata> {
        let tools = self.tools.read().unwrap();
        tools.get(name).map(|tool| tool.metadata().clone())
    }
    
    /// Find tools by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools.iter()
            .filter(|(_, tool)| tool.metadata().tags.contains(&tag.to_string()))
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// Find enabled tools
    pub fn enabled_tools(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools.iter()
            .filter(|(_, tool)| tool.metadata().enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        let tools = self.tools.read().unwrap();
        tools.len()
    }
    
    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        let tools = self.tools.read().unwrap();
        tools.is_empty()
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, name: &str) -> bool {
        let tools = self.tools.read().unwrap();
        tools.contains_key(name)
    }
    
    /// Clear all registered tools
    pub fn clear(&self) {
        let mut tools = self.tools.write().unwrap();
        let mut type_registry = self.type_registry.write().unwrap();
        tools.clear();
        type_registry.clear();
    }
    
    /// Execute a tool by name with JSON input
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> ToolResult<serde_json::Value> {
        let tool = self.get(name)
            .ok_or_else(|| error_context().with_tool(name).tool_not_found())?;
        
        tool.execute_json(input).await
    }
    
    /// Get input schema for a tool
    pub fn input_schema(&self, name: &str) -> Option<serde_json::Value> {
        let tools = self.tools.read().unwrap();
        tools.get(name).map(|tool| tool.input_schema())
    }
}

/// Iterator over tool names
pub struct ToolNameIter {
    names: std::vec::IntoIter<String>,
}

impl Iterator for ToolNameIter {
    type Item = String;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.names.next()
    }
}

impl IntoIterator for &ToolRegistry {
    type Item = String;
    type IntoIter = ToolNameIter;
    
    fn into_iter(self) -> Self::IntoIter {
        ToolNameIter {
            names: self.tool_names().into_iter(),
        }
    }
}



/// Global tool registry instance
static GLOBAL_REGISTRY: std::sync::OnceLock<ToolRegistry> = std::sync::OnceLock::new();

/// Get the global tool registry
pub fn global_registry() -> &'static ToolRegistry {
    GLOBAL_REGISTRY.get_or_init(ToolRegistry::new)
}

/// Register a tool in the global registry
pub fn register_global<T, I, O>(tool: T) -> ToolResult<()>
where
    T: Tool<I, O> + Clone + 'static,
    I: ToolInput,
    O: ToolOutput,
{
    global_registry().register(tool)?;
    Ok(())
}

/// Execute a tool from the global registry
pub async fn execute_global(name: &str, input: serde_json::Value) -> ToolResult<serde_json::Value> {
    global_registry().execute(name, input).await
}
