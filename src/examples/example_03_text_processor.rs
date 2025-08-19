// File: src/examples/example_03_text_processor.rs
//
// This example demonstrates a more complex server with multiple related tools
// for text processing operations. It shows how to organize multiple tools
// within a MCP server.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Request structures for different text operations
#[derive(Serialize, Deserialize, Debug)]
pub struct TextTransformRequest {
    pub text: String,
    pub operation: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextAnalysisRequest {
    pub text: String,
}

// Response structures
#[derive(Serialize, Deserialize, Debug)]
pub struct TextResponse {
    pub result: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextAnalysisResponse {
    pub word_count: usize,
    pub character_count: usize,
    pub line_count: usize,
    pub has_uppercase: bool,
    pub has_lowercase: bool,
    pub has_numbers: bool,
}

// Tool metadata structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// The text processing server with multiple related tools
pub struct TextProcessorServer;

impl Default for TextProcessorServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextProcessorServer {
    pub fn new() -> Self {
        Self
    }

    // Helper method for text transformation operations
    fn transform_text(&self, text: &str, operation: &str) -> Result<String, String> {
        match operation {
            "uppercase" => Ok(text.to_uppercase()),
            "lowercase" => Ok(text.to_lowercase()),
            "reverse" => Ok(text.chars().rev().collect()),
            "capitalize" => Ok(self.capitalize_words(text)),
            "trim" => Ok(text.trim().to_string()),
            _ => Err(format!("Unsupported transformation: {}", operation)),
        }
    }

    // Helper method to capitalize first letter of each word
    fn capitalize_words(&self, text: &str) -> String {
        text.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    // Helper method for text analysis
    fn analyze_text(&self, text: &str) -> TextAnalysisResponse {
        TextAnalysisResponse {
            word_count: text.split_whitespace().count(),
            character_count: text.chars().count(),
            line_count: text.lines().count(),
            has_uppercase: text.chars().any(|c| c.is_uppercase()),
            has_lowercase: text.chars().any(|c| c.is_lowercase()),
            has_numbers: text.chars().any(|c| c.is_numeric()),
        }
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            // Text transformation tool
            Tool {
                name: "transform_text".to_string(),
                description: "Transform text using various operations".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to transform"
                        },
                        "operation": {
                            "type": "string",
                            "description": "The transformation to apply",
                            "enum": ["uppercase", "lowercase", "reverse", "capitalize", "trim"]
                        }
                    },
                    "required": ["text", "operation"]
                }),
            },
            // Text analysis tool
            Tool {
                name: "analyze_text".to_string(),
                description: "Analyze text and provide statistics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to analyze"
                        }
                    },
                    "required": ["text"]
                }),
            },
        ]
    }

    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "transform_text" => {
                let request: TextTransformRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                let result = self.transform_text(&request.text, &request.operation)?;

                let response = TextResponse { result };
                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            "analyze_text" => {
                let request: TextAnalysisRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                let response = self.analyze_text(&request.text);
                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("📝 Starting Text Processor MCP Server");
    eprintln!("🛠️  Available tools: transform_text, analyze_text");
    eprintln!("💡 Send JSON-RPC messages via stdin");

    // Simple demo mode for testing
    let server = TextProcessorServer::new();

    // Demo usage
    eprintln!("\n🧪 Running demo transformations:");

    let demo_text = "hello world";
    let transform_args = serde_json::json!({
        "text": demo_text,
        "operation": "uppercase"
    });

    match server.call_tool("transform_text", transform_args) {
        Ok(result) => {
            let response: TextResponse = serde_json::from_value(result).unwrap();
            eprintln!(
                "✅ Transform '{}' to uppercase: '{}'",
                demo_text, response.result
            );
        }
        Err(e) => eprintln!("❌ Transform failed: {}", e),
    }

    let analyze_args = serde_json::json!({
        "text": demo_text
    });

    match server.call_tool("analyze_text", analyze_args) {
        Ok(result) => {
            let response: TextAnalysisResponse = serde_json::from_value(result).unwrap();
            eprintln!(
                "✅ Analysis of '{}': {} words, {} chars",
                demo_text, response.word_count, response.character_count
            );
        }
        Err(e) => eprintln!("❌ Analysis failed: {}", e),
    }

    eprintln!("\n🎉 Text processor demo completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_transformations() {
        let server = TextProcessorServer::new();

        // Test uppercase transformation
        let args = serde_json::json!({
            "text": "hello world",
            "operation": "uppercase"
        });

        let result = server.call_tool("transform_text", args).unwrap();
        let response: TextResponse = serde_json::from_value(result).unwrap();
        assert_eq!(response.result, "HELLO WORLD");

        // Test reverse transformation
        let args = serde_json::json!({
            "text": "hello",
            "operation": "reverse"
        });

        let result = server.call_tool("transform_text", args).unwrap();
        let response: TextResponse = serde_json::from_value(result).unwrap();
        assert_eq!(response.result, "olleh");
    }

    #[test]
    fn test_text_analysis() {
        let server = TextProcessorServer::new();

        let args = serde_json::json!({
            "text": "Hello World 123"
        });

        let result = server.call_tool("analyze_text", args).unwrap();
        let response: TextAnalysisResponse = serde_json::from_value(result).unwrap();

        assert_eq!(response.word_count, 3);
        assert_eq!(response.character_count, 15);
        assert!(response.has_uppercase);
        assert!(response.has_lowercase);
        assert!(response.has_numbers);
    }

    #[test]
    fn test_tool_listing() {
        let server = TextProcessorServer::new();
        let tools = server.list_tools();

        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t.name == "transform_text"));
        assert!(tools.iter().any(|t| t.name == "analyze_text"));
    }
}
