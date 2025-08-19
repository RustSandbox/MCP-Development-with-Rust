// File: src/examples/example_01_hello_world.rs
//
// This is the simplest possible MCP server implementation.
// It demonstrates the basic structure and initialization process
// for an MCP server using the official rust-sdk.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{stdin, stdout};

// Step 1: Define the request structure for our greeting tool.
// This struct represents the data that clients will send when calling our tool.
// The `Serialize` and `Deserialize` traits enable automatic JSON conversion.
#[derive(Serialize, Deserialize, Debug)]
pub struct GreetingRequest {
    // The name parameter that the tool will use for personalized greetings
    pub name: String,
}

// Step 2: Define the response structure for our greeting tool.
// This struct represents the data that our server will return to clients.
#[derive(Serialize, Deserialize, Debug)]
pub struct GreetingResponse {
    // The formatted greeting message to return to the client
    pub message: String,
}

// Step 3: Define tool metadata structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Step 4: Create our MCP server handler struct.
// This struct will handle MCP protocol messages.
pub struct HelloWorldServer;

impl Default for HelloWorldServer {
    fn default() -> Self {
        Self::new()
    }
}

impl HelloWorldServer {
    pub fn new() -> Self {
        Self
    }

    // Handle tool list requests - this tells clients what tools are available
    pub fn list_tools(&self) -> Vec<Tool> {
        // Return a list containing our single greeting tool
        // Each tool has a name, description, and input schema
        vec![Tool {
            name: "greeting".to_string(),
            description: "Generate a personalized greeting message".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the person to greet"
                    }
                },
                "required": ["name"]
            }),
        }]
    }

    // Handle tool call requests - this is where the actual tool logic executes
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "greeting" => {
                // Step 5: Parse the incoming request parameters
                let request: GreetingRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                // Step 6: Execute the tool logic (create a greeting)
                let response = GreetingResponse {
                    message: format!("Hello, {}! Welcome to MCP with Rust!", request.name),
                };

                // Step 7: Return the response as JSON
                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // Simple JSON-RPC message handler for demonstration
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

// Step 8: Main function to start the MCP server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to help with debugging
    tracing_subscriber::fmt::init();

    eprintln!("🚀 Starting Hello World MCP Server");
    eprintln!("📝 Available tools: greeting");
    eprintln!("💡 Send JSON-RPC messages via stdin");
    eprintln!("📋 Example: {{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}}");
    eprintln!();

    // Create our server handler instance
    let server = HelloWorldServer::new();

    // Simple message loop for demonstration
    // In a real implementation, this would be handled by the rmcp crate
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

    eprintln!("👋 Hello World server shutting down");
    Ok(())
}
