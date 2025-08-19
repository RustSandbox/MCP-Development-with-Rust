// File: src/examples/example_04_simple_client.rs
//
// This example demonstrates how to build an MCP client that can connect to
// and interact with MCP servers. It shows the client-side perspective of
// the MCP protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Structure to represent an MCP client application
pub struct SimpleMcpClient {
    // This simulates a connection to an MCP server
    server_url: String,
}

// Structures for client-server communication
#[derive(Serialize, Deserialize, Debug)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCallRequest {
    pub tool_name: String,
    pub arguments: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCallResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl SimpleMcpClient {
    // Constructor to create a new MCP client instance
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
        }
    }

    // Simulate connecting to an MCP server
    pub async fn connect(&self) -> Result<(), String> {
        eprintln!("🔗 Connecting to MCP server: {}", self.server_url);

        // In a real implementation, this would establish a connection
        // For this demo, we'll just simulate success
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        eprintln!("✅ Connected successfully!");
        Ok(())
    }

    // Simulate listing available tools from the server
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, String> {
        eprintln!("🔍 Discovering available tools...");

        // Simulate network delay
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Return mock tools for demonstration
        let tools = vec![
            ToolInfo {
                name: "greeting".to_string(),
                description: "Generate a personalized greeting message".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }),
            },
            ToolInfo {
                name: "calculator".to_string(),
                description: "Perform basic arithmetic operations".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["operation", "a", "b"]
                }),
            },
            ToolInfo {
                name: "text_transform".to_string(),
                description: "Transform text using various operations".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {"type": "string"},
                        "operation": {"type": "string"}
                    },
                    "required": ["text", "operation"]
                }),
            },
        ];

        eprintln!("📋 Found {} tools", tools.len());
        for tool in &tools {
            eprintln!("  - {}: {}", tool.name, tool.description);
        }

        Ok(tools)
    }

    // Simulate calling a tool on the server
    pub async fn call_tool(&self, request: ToolCallRequest) -> Result<ToolCallResponse, String> {
        eprintln!("🔧 Calling tool: {}", request.tool_name);

        // Simulate network delay
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Simulate tool execution based on tool name
        match request.tool_name.as_str() {
            "greeting" => {
                let name = request
                    .arguments
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown");

                let result = serde_json::json!({
                    "message": format!("Hello, {}! This is from the MCP server.", name)
                });

                Ok(ToolCallResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                })
            }
            "calculator" => {
                let operation = request
                    .arguments
                    .get("operation")
                    .and_then(|o| o.as_str())
                    .unwrap_or("");
                let a = request
                    .arguments
                    .get("a")
                    .and_then(|a| a.as_f64())
                    .unwrap_or(0.0);
                let b = request
                    .arguments
                    .get("b")
                    .and_then(|b| b.as_f64())
                    .unwrap_or(0.0);

                let result = match operation {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" => {
                        if b == 0.0 {
                            return Ok(ToolCallResponse {
                                success: false,
                                result: None,
                                error: Some("Division by zero".to_string()),
                            });
                        }
                        a / b
                    }
                    _ => {
                        return Ok(ToolCallResponse {
                            success: false,
                            result: None,
                            error: Some(format!("Unknown operation: {}", operation)),
                        });
                    }
                };

                Ok(ToolCallResponse {
                    success: true,
                    result: Some(serde_json::json!({"result": result})),
                    error: None,
                })
            }
            "text_transform" => {
                let text = request
                    .arguments
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                let operation = request
                    .arguments
                    .get("operation")
                    .and_then(|o| o.as_str())
                    .unwrap_or("");

                let result = match operation {
                    "uppercase" => text.to_uppercase(),
                    "lowercase" => text.to_lowercase(),
                    "reverse" => text.chars().rev().collect(),
                    _ => format!("Unknown operation: {}", operation),
                };

                Ok(ToolCallResponse {
                    success: true,
                    result: Some(serde_json::json!({"result": result})),
                    error: None,
                })
            }
            _ => Ok(ToolCallResponse {
                success: false,
                result: None,
                error: Some(format!("Unknown tool: {}", request.tool_name)),
            }),
        }
    }

    // Demonstrate a complete client workflow
    pub async fn demonstrate_client_workflow(&self) -> Result<(), String> {
        eprintln!("🚀 Starting MCP Client Demonstration");
        eprintln!("====================================");

        // Step 1: Connect to server
        self.connect().await?;

        // Step 2: List available tools
        let tools = self.list_tools().await?;

        // Step 3: Call each tool with sample data
        eprintln!("\n🧪 Testing tools with sample data:");

        // Test greeting tool
        if tools.iter().any(|t| t.name == "greeting") {
            let request = ToolCallRequest {
                tool_name: "greeting".to_string(),
                arguments: serde_json::json!({"name": "Rust Developer"}),
            };

            match self.call_tool(request).await? {
                ToolCallResponse {
                    success: true,
                    result: Some(result),
                    ..
                } => {
                    eprintln!("✅ Greeting result: {}", result);
                }
                ToolCallResponse {
                    success: false,
                    error: Some(err),
                    ..
                } => {
                    eprintln!("❌ Greeting failed: {}", err);
                }
                _ => eprintln!("⚠️  Unexpected greeting response"),
            }
        }

        // Test calculator tool
        if tools.iter().any(|t| t.name == "calculator") {
            let request = ToolCallRequest {
                tool_name: "calculator".to_string(),
                arguments: serde_json::json!({
                    "operation": "add",
                    "a": 15.0,
                    "b": 27.0
                }),
            };

            match self.call_tool(request).await? {
                ToolCallResponse {
                    success: true,
                    result: Some(result),
                    ..
                } => {
                    eprintln!("✅ Calculator result: {}", result);
                }
                ToolCallResponse {
                    success: false,
                    error: Some(err),
                    ..
                } => {
                    eprintln!("❌ Calculator failed: {}", err);
                }
                _ => eprintln!("⚠️  Unexpected calculator response"),
            }
        }

        // Test text transform tool
        if tools.iter().any(|t| t.name == "text_transform") {
            let request = ToolCallRequest {
                tool_name: "text_transform".to_string(),
                arguments: serde_json::json!({
                    "text": "Model Context Protocol",
                    "operation": "uppercase"
                }),
            };

            match self.call_tool(request).await? {
                ToolCallResponse {
                    success: true,
                    result: Some(result),
                    ..
                } => {
                    eprintln!("✅ Text transform result: {}", result);
                }
                ToolCallResponse {
                    success: false,
                    error: Some(err),
                    ..
                } => {
                    eprintln!("❌ Text transform failed: {}", err);
                }
                _ => eprintln!("⚠️  Unexpected text transform response"),
            }
        }

        eprintln!("\n🎉 Client demonstration completed successfully!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for better debugging
    tracing_subscriber::fmt::init();

    // Create a client instance
    let client = SimpleMcpClient::new("ws://localhost:8080");

    // Run the demonstration
    client.demonstrate_client_workflow().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SimpleMcpClient::new("ws://localhost:8080");
        assert_eq!(client.server_url, "ws://localhost:8080");
    }

    #[tokio::test]
    async fn test_list_tools() {
        let client = SimpleMcpClient::new("test://server");
        let tools = client.list_tools().await.unwrap();

        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.name == "greeting"));
        assert!(tools.iter().any(|t| t.name == "calculator"));
        assert!(tools.iter().any(|t| t.name == "text_transform"));
    }

    #[tokio::test]
    async fn test_tool_calls() {
        let client = SimpleMcpClient::new("test://server");

        // Test greeting tool
        let greeting_request = ToolCallRequest {
            tool_name: "greeting".to_string(),
            arguments: serde_json::json!({"name": "Test User"}),
        };

        let response = client.call_tool(greeting_request).await.unwrap();
        assert!(response.success);
        assert!(response.result.is_some());

        // Test calculator tool
        let calc_request = ToolCallRequest {
            tool_name: "calculator".to_string(),
            arguments: serde_json::json!({
                "operation": "multiply",
                "a": 6.0,
                "b": 7.0
            }),
        };

        let response = client.call_tool(calc_request).await.unwrap();
        assert!(response.success);
        assert!(response.result.is_some());

        // Test error case
        let error_request = ToolCallRequest {
            tool_name: "unknown_tool".to_string(),
            arguments: serde_json::json!({}),
        };

        let response = client.call_tool(error_request).await.unwrap();
        assert!(!response.success);
        assert!(response.error.is_some());
    }
}
