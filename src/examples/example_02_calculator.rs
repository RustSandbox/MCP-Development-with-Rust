// File: src/examples/example_02_calculator.rs
//
// This example builds upon the hello world server by adding a calculator tool
// that demonstrates parameter validation, error handling, and multiple operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{stdin, stdout};

// Define the calculator request structure with multiple parameters
#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatorRequest {
    // The mathematical operation to perform
    pub operation: String,
    // First number in the calculation
    pub a: f64,
    // Second number in the calculation
    pub b: f64,
}

// Define the calculator response structure
#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatorResponse {
    // The result of the calculation
    pub result: f64,
    // The operation that was performed (for confirmation)
    pub operation_performed: String,
}

// Custom error type for calculator-specific errors
#[derive(Debug)]
pub enum CalculatorError {
    DivisionByZero,
    UnsupportedOperation(String),
}

impl std::fmt::Display for CalculatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalculatorError::DivisionByZero => write!(f, "Division by zero is not allowed"),
            CalculatorError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
        }
    }
}

impl std::error::Error for CalculatorError {}

// Tool metadata structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// The calculator server handler
pub struct CalculatorServer;

impl Default for CalculatorServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorServer {
    pub fn new() -> Self {
        Self
    }

    // Private method to perform the actual calculation
    // This demonstrates separation of concerns and clean code principles
    fn perform_calculation(&self, request: &CalculatorRequest) -> Result<f64, CalculatorError> {
        match request.operation.as_str() {
            "add" => Ok(request.a + request.b),
            "subtract" => Ok(request.a - request.b),
            "multiply" => Ok(request.a * request.b),
            "divide" => {
                // Validate that we're not dividing by zero
                if request.b == 0.0 {
                    Err(CalculatorError::DivisionByZero)
                } else {
                    Ok(request.a / request.b)
                }
            }
            _ => Err(CalculatorError::UnsupportedOperation(
                request.operation.clone(),
            )),
        }
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "calculator".to_string(),
            description: "Perform basic arithmetic operations (add, subtract, multiply, divide)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "The operation to perform",
                        "enum": ["add", "subtract", "multiply", "divide"]
                    },
                    "a": {
                        "type": "number",
                        "description": "First number"
                    },
                    "b": {
                        "type": "number",
                        "description": "Second number"
                    }
                },
                "required": ["operation", "a", "b"]
            }),
        }]
    }

    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "calculator" => {
                // Parse the request
                let request: CalculatorRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                // Perform the calculation
                let result = self
                    .perform_calculation(&request)
                    .map_err(|e| e.to_string())?;

                // Create the response
                let response = CalculatorResponse {
                    result,
                    operation_performed: format!(
                        "{} {} {}",
                        request.a, request.operation, request.b
                    ),
                };

                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // Simple JSON-RPC message handler
    pub fn handle_message(&self, message: Value) -> Result<Value, String> {
        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or("Missing method")?;

        match method {
            "tools/list" => {
                let tools = self.list_tools();
                Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id"),
                    "result": {
                        "tools": tools
                    }
                }))
            }
            "tools/call" => {
                let params = message.get("params").ok_or("Missing params")?;

                let tool_name = params
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or("Missing tool name")?;

                let arguments = params
                    .get("arguments")
                    .unwrap_or(&Value::Object(serde_json::Map::new()))
                    .clone();

                match self.call_tool(tool_name, arguments) {
                    Ok(result) => Ok(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id"),
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string(&result).unwrap_or_default()
                            }]
                        }
                    })),
                    Err(error) => Ok(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id"),
                        "error": {
                            "code": -32000,
                            "message": error
                        }
                    })),
                }
            }
            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("🧮 Starting Calculator MCP Server");
    eprintln!("📝 Available tools: calculator");
    eprintln!("💡 Send JSON-RPC messages via stdin");
    eprintln!("📋 Example: {{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"calculator\",\"arguments\":{{\"operation\":\"add\",\"a\":5,\"b\":3}}}}}}");
    eprintln!();

    let server = CalculatorServer::new();

    // Message loop for JSON-RPC communication
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let stdin = stdin();
    let mut stdout = stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(trimmed) {
                    Ok(message) => match server.handle_message(message) {
                        Ok(response) => {
                            let response_str = serde_json::to_string(&response)?;
                            stdout.write_all(response_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Err(e) => {
                            eprintln!("Error handling message: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    eprintln!("🧮 Calculator server shutting down");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_operations() {
        let server = CalculatorServer::new();

        // Test addition
        let add_args = serde_json::json!({
            "operation": "add",
            "a": 5.0,
            "b": 3.0
        });

        let result = server.call_tool("calculator", add_args).unwrap();
        let response: CalculatorResponse = serde_json::from_value(result).unwrap();
        assert_eq!(response.result, 8.0);

        // Test division by zero
        let div_zero_args = serde_json::json!({
            "operation": "divide",
            "a": 5.0,
            "b": 0.0
        });

        let result = server.call_tool("calculator", div_zero_args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_tool_listing() {
        let server = CalculatorServer::new();
        let tools = server.list_tools();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "calculator");
    }
}
