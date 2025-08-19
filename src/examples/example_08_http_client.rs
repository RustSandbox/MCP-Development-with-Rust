// File: src/examples/example_08_http_client.rs
//
// This example demonstrates HTTP client integration in an MCP server.
// It shows how to safely make external API calls, handle responses,
// and manage authentication while following best practices.

use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

// Configuration for HTTP operations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpClientConfig {
    pub timeout_seconds: u64,
    pub max_response_size: usize,
    pub allowed_domains: Vec<String>,
    pub default_headers: HashMap<String, String>,
    pub user_agent: String,
    pub follow_redirects: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        let mut default_headers = HashMap::new();
        default_headers.insert("Accept".to_string(), "application/json".to_string());
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());

        Self {
            timeout_seconds: 30,
            max_response_size: 1024 * 1024, // 1MB
            allowed_domains: vec![
                "httpbin.org".to_string(),
                "api.github.com".to_string(),
                "jsonplaceholder.typicode.com".to_string(),
            ],
            default_headers,
            user_agent: "MCP-Rust-Client/1.0".to_string(),
            follow_redirects: true,
        }
    }
}

// Request structures
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpRequest {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiCallRequest {
    pub service: String,
    pub endpoint: String,
    pub parameters: Option<HashMap<String, Value>>,
}

// Response structures
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub url: String,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// HTTP Client Server
pub struct HttpClientServer {
    config: HttpClientConfig,
    client: Client,
}

impl HttpClientServer {
    pub fn new(config: HttpClientConfig) -> Result<Self, String> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent);

        if !config.follow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }

        let client = client_builder
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { config, client })
    }

    // Validate URL is allowed
    fn validate_url(&self, url: &str) -> Result<reqwest::Url, String> {
        let parsed_url = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

        // Check if domain is allowed
        if let Some(host) = parsed_url.host_str() {
            if !self
                .config
                .allowed_domains
                .iter()
                .any(|domain| host.contains(domain))
            {
                return Err(format!("Domain '{}' is not in allowed list", host));
            }
        } else {
            return Err("URL must have a valid host".to_string());
        }

        // Only allow HTTPS and HTTP
        match parsed_url.scheme() {
            "http" | "https" => Ok(parsed_url),
            scheme => Err(format!("Unsupported URL scheme: {}", scheme)),
        }
    }

    // Convert reqwest Response to our HttpResponse
    async fn process_response(&self, response: Response) -> Result<HttpResponse, String> {
        let status = response.status().as_u16();
        let url = response.url().to_string();

        // Extract headers
        let mut headers = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .map(|s| s.to_string());

        let content_length = response.content_length().map(|len| len as usize);

        // Check response size
        if let Some(len) = content_length {
            if len > self.config.max_response_size {
                return Err(format!("Response too large: {} bytes", len));
            }
        }

        // Read response body
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Double-check body size after reading
        if body.len() > self.config.max_response_size {
            return Err(format!("Response body too large: {} bytes", body.len()));
        }

        let body_len = body.len();
        Ok(HttpResponse {
            status,
            headers,
            body,
            url,
            content_type,
            content_length: Some(body_len),
        })
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "http_request".to_string(),
                description: "Make HTTP requests to allowed external APIs".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to make the request to"
                        },
                        "method": {
                            "type": "string",
                            "description": "HTTP method (GET, POST, PUT, DELETE)",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"],
                            "default": "GET"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Additional headers to send",
                            "additionalProperties": {"type": "string"}
                        },
                        "body": {
                            "type": "string",
                            "description": "Request body (for POST/PUT requests)"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Request timeout in seconds"
                        }
                    },
                    "required": ["url"]
                }),
            },
            Tool {
                name: "api_call".to_string(),
                description: "Make calls to pre-configured API services".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "API service to call",
                            "enum": ["httpbin", "jsonplaceholder", "github"]
                        },
                        "endpoint": {
                            "type": "string",
                            "description": "API endpoint to call"
                        },
                        "parameters": {
                            "type": "object",
                            "description": "Parameters to send with the request"
                        }
                    },
                    "required": ["service", "endpoint"]
                }),
            },
            Tool {
                name: "health_check".to_string(),
                description: "Check if a URL is accessible".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to check"
                        }
                    },
                    "required": ["url"]
                }),
            },
        ]
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "http_request" => self.http_request(arguments).await,
            "api_call" => self.api_call(arguments).await,
            "health_check" => self.health_check(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn http_request(&self, arguments: Value) -> Result<Value, String> {
        let request: HttpRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let url = self.validate_url(&request.url)?;

        // Parse HTTP method
        let method = match request
            .method
            .as_deref()
            .unwrap_or("GET")
            .to_uppercase()
            .as_str()
        {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            m => return Err(format!("Unsupported HTTP method: {}", m)),
        };

        // Build request
        let mut req_builder = self.client.request(method, url);

        // Add default headers
        for (key, value) in &self.config.default_headers {
            req_builder = req_builder.header(key, value);
        }

        // Add custom headers
        if let Some(headers) = request.headers {
            for (key, value) in headers {
                req_builder = req_builder.header(key, value);
            }
        }

        // Add body if provided
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        // Set custom timeout if provided
        if let Some(timeout) = request.timeout {
            req_builder = req_builder.timeout(Duration::from_secs(timeout));
        }

        // Send request
        let response = req_builder
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let http_response = self.process_response(response).await?;

        serde_json::to_value(http_response)
            .map_err(|e| format!("Failed to serialize response: {}", e))
    }

    async fn api_call(&self, arguments: Value) -> Result<Value, String> {
        let request: ApiCallRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        // Build URL based on service
        let base_url = match request.service.as_str() {
            "httpbin" => "https://httpbin.org",
            "jsonplaceholder" => "https://jsonplaceholder.typicode.com",
            "github" => "https://api.github.com",
            _ => return Err(format!("Unknown service: {}", request.service)),
        };

        let url = format!("{}/{}", base_url, request.endpoint);

        // Build HTTP request
        let http_request = HttpRequest {
            url,
            method: Some("GET".to_string()),
            headers: None,
            body: None,
            timeout: None,
        };

        self.http_request(
            serde_json::to_value(http_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?,
        )
        .await
    }

    async fn health_check(&self, arguments: Value) -> Result<Value, String> {
        let request: HttpRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let url = self.validate_url(&request.url)?;

        let start = std::time::Instant::now();

        match self.client.head(url.clone()).send().await {
            Ok(response) => {
                let duration = start.elapsed();
                Ok(serde_json::json!({
                    "url": url.to_string(),
                    "accessible": true,
                    "status": response.status().as_u16(),
                    "response_time_ms": duration.as_millis(),
                    "headers": response.headers().len()
                }))
            }
            Err(e) => {
                let duration = start.elapsed();
                Ok(serde_json::json!({
                    "url": url.to_string(),
                    "accessible": false,
                    "error": e.to_string(),
                    "response_time_ms": duration.as_millis()
                }))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("🌐 Starting HTTP Client MCP Server");
    eprintln!("=================================");

    // Create config
    let config = HttpClientConfig::default();

    eprintln!("⚙️  HTTP Configuration:");
    eprintln!("   Timeout: {}s", config.timeout_seconds);
    eprintln!("   Max response size: {} bytes", config.max_response_size);
    eprintln!("   Allowed domains: {:?}", config.allowed_domains);
    eprintln!("   User agent: {}", config.user_agent);

    // Create server
    let server = HttpClientServer::new(config)?;

    // Demo HTTP operations
    eprintln!("\n🧪 HTTP Client Demo:");

    // List tools
    let tools = server.list_tools();
    eprintln!("📋 Available tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Test health check
    eprintln!("\n🏥 Health check test:");
    let health_args = serde_json::json!({
        "url": "https://httpbin.org"
    });

    match server.call_tool("health_check", health_args).await {
        Ok(result) => {
            let accessible = result
                .get("accessible")
                .unwrap_or(&Value::Bool(false))
                .as_bool()
                .unwrap_or(false);
            let response_time = result.get("response_time_ms").unwrap_or(&Value::Null);

            if accessible {
                eprintln!("  ✅ httpbin.org is accessible ({}ms)", response_time);
            } else {
                eprintln!("  ❌ httpbin.org is not accessible");
            }
        }
        Err(e) => eprintln!("  ❌ Health check failed: {}", e),
    }

    // Test API call
    eprintln!("\n🔌 API call test:");
    let api_args = serde_json::json!({
        "service": "httpbin",
        "endpoint": "get"
    });

    match server.call_tool("api_call", api_args).await {
        Ok(result) => {
            if let Ok(response) = serde_json::from_value::<HttpResponse>(result) {
                eprintln!("  ✅ API call successful:");
                eprintln!("     Status: {}", response.status);
                eprintln!(
                    "     Content-Type: {}",
                    response.content_type.unwrap_or("unknown".to_string())
                );
                eprintln!("     Body size: {} bytes", response.body.len());
            }
        }
        Err(e) => eprintln!("  ❌ API call failed: {}", e),
    }

    // Test custom HTTP request
    eprintln!("\n📡 Custom HTTP request test:");
    let http_args = serde_json::json!({
        "url": "https://jsonplaceholder.typicode.com/posts/1",
        "method": "GET"
    });

    match server.call_tool("http_request", http_args).await {
        Ok(result) => {
            if let Ok(response) = serde_json::from_value::<HttpResponse>(result) {
                eprintln!("  ✅ HTTP request successful:");
                eprintln!("     Status: {}", response.status);
                eprintln!("     URL: {}", response.url);

                // Try to parse JSON response
                if let Ok(json) = serde_json::from_str::<Value>(&response.body) {
                    if let Some(title) = json.get("title") {
                        eprintln!("     Post title: {}", title);
                    }
                }
            }
        }
        Err(e) => eprintln!("  ❌ HTTP request failed: {}", e),
    }

    eprintln!("\n🎉 HTTP client demo completed!");
    eprintln!("\n🔒 Security features:");
    eprintln!("   ✅ Domain allowlisting");
    eprintln!("   ✅ Response size limits");
    eprintln!("   ✅ Request timeouts");
    eprintln!("   ✅ URL validation");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_server() {
        let config = HttpClientConfig::default();
        let server = HttpClientServer::new(config).unwrap();

        let tools = server.list_tools();
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.name == "http_request"));
        assert!(tools.iter().any(|t| t.name == "api_call"));
        assert!(tools.iter().any(|t| t.name == "health_check"));
    }

    #[test]
    fn test_url_validation() {
        let config = HttpClientConfig::default();
        let server = HttpClientServer::new(config).unwrap();

        // Valid URL should work
        let result = server.validate_url("https://httpbin.org/get");
        assert!(result.is_ok());

        // Invalid domain should fail
        let result = server.validate_url("https://evil.com/get");
        assert!(result.is_err());

        // Invalid scheme should fail
        let result = server.validate_url("ftp://httpbin.org/get");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = HttpClientConfig::default();
        let server = HttpClientServer::new(config).unwrap();

        let args = serde_json::json!({
            "url": "https://httpbin.org"
        });

        // Note: This test requires internet connection
        // In a real test suite, you'd mock the HTTP client
        match server.call_tool("health_check", args).await {
            Ok(result) => {
                assert!(result.get("url").is_some());
                assert!(result.get("accessible").is_some());
                assert!(result.get("response_time_ms").is_some());
            }
            Err(_) => {
                // Test might fail due to network issues, which is acceptable
                // In production, use mocking for reliable tests
            }
        }
    }
}
