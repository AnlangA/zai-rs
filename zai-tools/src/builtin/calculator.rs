//! Calculator tool for mathematical operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::{Tool, ToolInput, ToolOutput, ToolMetadata};
use crate::error::{ToolResult, error_context};

/// Input parameters for calculator operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatorInput {
    /// Mathematical operation to perform
    pub operation: String,
    /// First operand
    pub a: f64,
    /// Second operand
    pub b: f64,
}

/// Calculator result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatorOutput {
    /// The calculated result
    pub result: f64,
    /// Human-readable expression
    pub expression: String,
    /// The operation that was performed
    pub operation: String,
}

// Implement the required traits
impl ToolInput for CalculatorInput {
    fn validate(&self) -> ToolResult<()> {
        match self.operation.as_str() {
            "add" | "subtract" | "multiply" | "divide" | "power" | "sqrt" | "abs" => Ok(()),
            _ => Err(error_context()
                .invalid_parameters(format!("Unsupported operation: {}. Supported operations: add, subtract, multiply, divide, power, sqrt, abs", self.operation))),
        }
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide", "power", "sqrt", "abs"],
                    "description": "Mathematical operation to perform"
                },
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand (not used for sqrt and abs operations)"
                }
            },
            "required": ["operation", "a"],
            "additionalProperties": false
        })
    }
}

impl ToolOutput for CalculatorOutput {}

/// Calculator tool for basic mathematical operations
#[derive(Debug, Clone)]
pub struct CalculatorTool {
    metadata: ToolMetadata,
}

impl CalculatorTool {
    /// Create a new calculator tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata::new::<CalculatorInput, CalculatorOutput>(
                "calculator",
                "Perform mathematical operations including basic arithmetic, power, square root, and absolute value"
            )
            .version("2.0.0")
            .author("ZAI Tools")
            .tags(["math", "calculator", "arithmetic"]),
        }
    }
}

impl Default for CalculatorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool<CalculatorInput, CalculatorOutput> for CalculatorTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, input: CalculatorInput) -> ToolResult<CalculatorOutput> {
        // Perform calculation
        let result = match input.operation.to_lowercase().as_str() {
            "add" => input.a + input.b,
            "subtract" => input.a - input.b,
            "multiply" => input.a * input.b,
            "divide" => {
                if input.b == 0.0 {
                    return Err(error_context()
                        .with_tool("calculator")
                        .execution_failed("Division by zero"));
                }
                input.a / input.b
            }
            "power" => input.a.powf(input.b),
            "sqrt" => {
                if input.a < 0.0 {
                    return Err(error_context()
                        .with_tool("calculator")
                        .execution_failed("Cannot calculate square root of negative number"));
                }
                input.a.sqrt()
            }
            "abs" => input.a.abs(),
            _ => unreachable!(), // Validation should catch this
        };

        let expression = match input.operation.as_str() {
            "sqrt" => format!("sqrt({})", input.a),
            "abs" => format!("abs({})", input.a),
            _ => format!("{} {} {}", input.a, input.operation, input.b),
        };

        Ok(CalculatorOutput {
            result,
            expression,
            operation: input.operation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculator_add() {
        let tool = CalculatorTool::new();
        let input = CalculatorInput {
            operation: "add".to_string(),
            a: 5.0,
            b: 3.0,
        };

        let result = tool.execute(input).await.unwrap();
        assert_eq!(result.result, 8.0);
        assert_eq!(result.operation, "add");
        assert_eq!(result.expression, "5 add 3");
    }

    #[tokio::test]
    async fn test_calculator_divide_by_zero() {
        let tool = CalculatorTool::new();
        let input = CalculatorInput {
            operation: "divide".to_string(),
            a: 5.0,
            b: 0.0,
        };

        let result = tool.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calculator_sqrt() {
        let tool = CalculatorTool::new();
        let input = CalculatorInput {
            operation: "sqrt".to_string(),
            a: 16.0,
            b: 0.0, // Not used for sqrt
        };

        let result = tool.execute(input).await.unwrap();
        assert_eq!(result.result, 4.0);
        assert_eq!(result.expression, "sqrt(16)");
    }

    #[tokio::test]
    async fn test_calculator_validation() {
        let input = CalculatorInput {
            operation: "invalid".to_string(),
            a: 5.0,
            b: 3.0,
        };

        let result = input.validate();
        assert!(result.is_err());
    }
}
