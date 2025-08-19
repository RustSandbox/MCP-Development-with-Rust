// File: src/examples/example_05_resource_provider.rs
//
// This example demonstrates how to implement MCP resources, which allow
// servers to provide data and content that LLMs can access. Resources
// are identified by URIs and can contain text or binary data.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// Structure representing a simple document resource
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    pub created_at: String,
    pub tags: Vec<String>,
}

// Structure representing an MCP resource
#[derive(Serialize, Deserialize, Debug)]
pub struct Resource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

// Request structure for document search
#[derive(Serialize, Deserialize, Debug)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

// Response structure for document search
#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResponse {
    pub matches: Vec<DocumentSummary>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentSummary {
    pub id: String,
    pub title: String,
    pub author: String,
    pub uri: String,
    pub tags: Vec<String>,
}

// Structure for tool definitions
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// The resource provider server
pub struct ResourceProviderServer {
    // In-memory document storage for this example
    // In a real application, this might be a database connection
    documents: HashMap<String, Document>,
}

impl Default for ResourceProviderServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceProviderServer {
    // Constructor to create a new server with sample documents
    pub fn new() -> Self {
        let mut documents = HashMap::new();

        // Add comprehensive sample documents for testing
        documents.insert("doc1".to_string(), Document {
            id: "doc1".to_string(),
            title: "Introduction to Model Context Protocol".to_string(),
            content: "The Model Context Protocol (MCP) is an open protocol that standardizes how applications provide context to LLMs. It enables a new class of AI-powered tools and workflows by solving the M x N integration problem. Instead of building custom integrations for each combination of LLM application and data source, MCP provides a standardized way to connect any LLM application to any data source through a unified protocol.".to_string(),
            author: "MCP Team".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            tags: vec!["MCP".to_string(), "Protocol".to_string(), "AI".to_string()],
        });

        documents.insert("doc2".to_string(), Document {
            id: "doc2".to_string(),
            title: "Rust Programming Language Overview".to_string(),
            content: "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It achieves these goals without requiring a garbage collector or a runtime. Rust's ownership system allows for memory safety without garbage collection, making it an excellent choice for system-level programming.".to_string(),
            author: "Rust Community".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
            tags: vec!["Rust".to_string(), "Programming".to_string(), "Systems".to_string()],
        });

        documents.insert("doc3".to_string(), Document {
            id: "doc3".to_string(),
            title: "Async Programming in Rust with Tokio".to_string(),
            content: "Async programming in Rust is powered by the Future trait and async/await syntax. The tokio runtime provides the infrastructure for running async code efficiently. Tokio is an asynchronous runtime for the Rust programming language that provides the building blocks needed for writing network applications.".to_string(),
            author: "Tokio Contributors".to_string(),
            created_at: "2024-01-03T00:00:00Z".to_string(),
            tags: vec!["Rust".to_string(), "Async".to_string(), "Tokio".to_string()],
        });

        documents.insert("doc4".to_string(), Document {
            id: "doc4".to_string(),
            title: "JSON-RPC 2.0 Specification".to_string(),
            content: "JSON-RPC is a stateless, light-weight remote procedure call (RPC) protocol. It is transport agnostic and can be used over HTTP, WebSockets, or other transports. The protocol defines how to encode requests and responses using JSON, making it language-independent and easy to implement.".to_string(),
            author: "JSON-RPC Working Group".to_string(),
            created_at: "2024-01-04T00:00:00Z".to_string(),
            tags: vec!["JSON-RPC".to_string(), "Protocol".to_string(), "API".to_string()],
        });

        Self { documents }
    }

    // List all available resources
    pub fn list_resources(&self) -> Vec<Resource> {
        self.documents
            .values()
            .map(|doc| Resource {
                uri: format!("document://{}", doc.id),
                name: Some(doc.title.clone()),
                description: Some(format!(
                    "Document by {} - Tags: {}",
                    doc.author,
                    doc.tags.join(", ")
                )),
                mime_type: Some("text/plain".to_string()),
            })
            .collect()
    }

    // Read a specific resource by URI
    pub fn read_resource(&self, uri: &str) -> Result<Value, String> {
        // Parse the URI to extract the document ID
        if let Some(doc_id) = uri.strip_prefix("document://") {
            if let Some(document) = self.documents.get(doc_id) {
                // Return the document content as a resource
                Ok(serde_json::json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/plain",
                        "text": document.content
                    }]
                }))
            } else {
                Err(format!("Document not found: {}", doc_id))
            }
        } else {
            Err(format!("Invalid document URI: {}", uri))
        }
    }

    // Helper method to search documents by query
    fn search_documents(&self, query: &str, limit: Option<usize>) -> Vec<&Document> {
        let query_lower = query.to_lowercase();
        let mut matches: Vec<&Document> = self
            .documents
            .values()
            .filter(|doc| {
                doc.title.to_lowercase().contains(&query_lower)
                    || doc.content.to_lowercase().contains(&query_lower)
                    || doc.author.to_lowercase().contains(&query_lower)
                    || doc
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect();

        // Sort by relevance (simple scoring based on title matches)
        matches.sort_by(|a, b| {
            let a_score = if a.title.to_lowercase().contains(&query_lower) {
                2
            } else {
                0
            } + if a
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower))
            {
                1
            } else {
                0
            };
            let b_score = if b.title.to_lowercase().contains(&query_lower) {
                2
            } else {
                0
            } + if b
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower))
            {
                1
            } else {
                0
            };
            b_score.cmp(&a_score)
        });

        if let Some(limit) = limit {
            matches.into_iter().take(limit).collect()
        } else {
            matches
        }
    }

    // Get document by ID
    fn get_document(&self, id: &str) -> Option<&Document> {
        self.documents.get(id)
    }

    // List available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "search_documents".to_string(),
                description: "Search through available documents using keywords".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query to find relevant documents"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "get_document_details".to_string(),
                description: "Get detailed information about a specific document".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "document_id": {
                            "type": "string",
                            "description": "ID of the document to retrieve details for"
                        }
                    },
                    "required": ["document_id"]
                }),
            },
        ]
    }

    // Handle tool calls
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "search_documents" => {
                let request: SearchRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                let matches = self.search_documents(&request.query, request.limit);

                let response = SearchResponse {
                    total_count: matches.len(),
                    matches: matches
                        .into_iter()
                        .map(|doc| DocumentSummary {
                            id: doc.id.clone(),
                            title: doc.title.clone(),
                            author: doc.author.clone(),
                            uri: format!("document://{}", doc.id),
                            tags: doc.tags.clone(),
                        })
                        .collect(),
                };

                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            "get_document_details" => {
                let document_id = arguments
                    .get("document_id")
                    .and_then(|id| id.as_str())
                    .ok_or("Missing document_id parameter")?;

                if let Some(document) = self.get_document(document_id) {
                    serde_json::to_value(document)
                        .map_err(|e| format!("Failed to serialize document: {}", e))
                } else {
                    Err(format!("Document not found: {}", document_id))
                }
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("📚 Starting Resource Provider MCP Server");
    eprintln!("🗂️  Sample documents with search capabilities loaded");
    eprintln!();

    let server = ResourceProviderServer::new();

    // Demonstrate resource functionality
    eprintln!("🧪 Demonstrating resource functionality:");

    // List resources
    let resources = server.list_resources();
    eprintln!("📋 Available resources ({} total):", resources.len());
    for resource in &resources {
        eprintln!(
            "  - {} ({})",
            resource.uri,
            resource.name.as_deref().unwrap_or("Unnamed")
        );
    }

    // Demonstrate search functionality
    eprintln!("\n🔍 Search demonstration:");
    let search_args = serde_json::json!({
        "query": "Rust",
        "limit": 3
    });

    match server.call_tool("search_documents", search_args) {
        Ok(result) => {
            let response: SearchResponse = serde_json::from_value(result).unwrap();
            eprintln!(
                "✅ Found {} documents matching 'Rust':",
                response.total_count
            );
            for doc in response.matches {
                eprintln!("  - {}: {} [{}]", doc.id, doc.title, doc.tags.join(", "));
            }
        }
        Err(e) => eprintln!("❌ Search failed: {}", e),
    }

    // Demonstrate resource reading
    eprintln!("\n📖 Resource reading demonstration:");
    match server.read_resource("document://doc1") {
        Ok(content) => {
            eprintln!("✅ Successfully read document://doc1");
            let text = content
                .get("contents")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("No content");
            eprintln!(
                "📄 Content preview: {}...",
                &text[..std::cmp::min(text.len(), 100)]
            );
        }
        Err(e) => eprintln!("❌ Read failed: {}", e),
    }

    eprintln!("\n🎉 Resource provider demonstration completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_listing() {
        let server = ResourceProviderServer::new();
        let resources = server.list_resources();

        assert_eq!(resources.len(), 4);
        assert!(resources.iter().any(|r| r.uri == "document://doc1"));
        assert!(resources.iter().any(|r| r.uri == "document://doc2"));
    }

    #[test]
    fn test_resource_reading() {
        let server = ResourceProviderServer::new();

        // Test valid resource
        let result = server.read_resource("document://doc1");
        assert!(result.is_ok());

        // Test invalid resource
        let result = server.read_resource("document://nonexistent");
        assert!(result.is_err());

        // Test invalid URI format
        let result = server.read_resource("invalid://doc1");
        assert!(result.is_err());
    }

    #[test]
    fn test_document_search() {
        let server = ResourceProviderServer::new();

        // Test search by title
        let search_args = serde_json::json!({
            "query": "Rust",
            "limit": 5
        });

        let result = server.call_tool("search_documents", search_args).unwrap();
        let response: SearchResponse = serde_json::from_value(result).unwrap();

        assert!(response.total_count > 0);
        assert!(response
            .matches
            .iter()
            .any(|doc| doc.title.contains("Rust")));
    }

    #[test]
    fn test_get_document_details() {
        let server = ResourceProviderServer::new();

        // Test valid document
        let args = serde_json::json!({"document_id": "doc1"});
        let result = server.call_tool("get_document_details", args);
        assert!(result.is_ok());

        // Test invalid document
        let args = serde_json::json!({"document_id": "nonexistent"});
        let result = server.call_tool("get_document_details", args);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_listing() {
        let server = ResourceProviderServer::new();
        let tools = server.list_tools();

        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t.name == "search_documents"));
        assert!(tools.iter().any(|t| t.name == "get_document_details"));
    }
}
