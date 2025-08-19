// File: src/examples/example_06_configurable_server.rs
//
// This example demonstrates how to build a configurable MCP server that can be
// customized through external configuration files, environment variables, and
// command-line arguments. This is essential for real-world deployments.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

// Configuration structure for our server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub server_name: String,
    pub version: String,
    pub max_connections: u32,
    pub timeout_seconds: u64,
    pub enabled_features: Vec<String>,
    pub tool_configs: HashMap<String, ToolConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolConfig {
    pub enabled: bool,
    pub description_override: Option<String>,
    pub parameters: HashMap<String, Value>,
}

// Default configuration
impl Default for ServerConfig {
    fn default() -> Self {
        let mut tool_configs = HashMap::new();

        tool_configs.insert(
            "greeting".to_string(),
            ToolConfig {
                enabled: true,
                description_override: None,
                parameters: HashMap::new(),
            },
        );

        tool_configs.insert(
            "echo".to_string(),
            ToolConfig {
                enabled: true,
                description_override: Some("Echo messages with configurable prefix".to_string()),
                parameters: [("prefix".to_string(), Value::String("Echo: ".to_string()))]
                    .iter()
                    .cloned()
                    .collect(),
            },
        );

        tool_configs.insert(
            "status".to_string(),
            ToolConfig {
                enabled: true,
                description_override: None,
                parameters: HashMap::new(),
            },
        );

        Self {
            server_name: "Configurable MCP Server".to_string(),
            version: "1.0.0".to_string(),
            max_connections: 100,
            timeout_seconds: 30,
            enabled_features: vec!["logging".to_string(), "metrics".to_string()],
            tool_configs,
        }
    }
}

// Tool structures
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GreetingRequest {
    pub name: String,
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EchoRequest {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
    pub server_name: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub active_connections: u32,
    pub enabled_features: Vec<String>,
    pub total_requests: u64,
}

// Configurable MCP Server
pub struct ConfigurableServer {
    config: ServerConfig,
    start_time: std::time::Instant,
    request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl ConfigurableServer {
    // Create server with configuration
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            start_time: std::time::Instant::now(),
            request_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    // Load configuration from multiple sources with priority:
    // 1. Command line arguments (highest priority)
    // 2. Environment variables
    // 3. Configuration file
    // 4. Default values (lowest priority)
    pub fn load_config() -> Result<ServerConfig, String> {
        // Start with default configuration
        let mut config = ServerConfig::default();

        // Try to load from config file if specified
        if let Ok(config_path) = env::var("MCP_CONFIG_FILE") {
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(file_config) = serde_json::from_str::<ServerConfig>(&config_content) {
                    config = file_config;
                    eprintln!("📋 Loaded configuration from: {}", config_path);
                }
            }
        }

        // Override with environment variables
        if let Ok(server_name) = env::var("MCP_SERVER_NAME") {
            config.server_name = server_name;
        }

        if let Ok(max_conn) = env::var("MCP_MAX_CONNECTIONS") {
            if let Ok(max_conn) = max_conn.parse::<u32>() {
                config.max_connections = max_conn;
            }
        }

        if let Ok(timeout) = env::var("MCP_TIMEOUT_SECONDS") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                config.timeout_seconds = timeout;
            }
        }

        // Override with command line arguments (simulated for demo)
        let args: Vec<String> = env::args().collect();
        for i in 0..args.len() {
            match args[i].as_str() {
                "--server-name" if i + 1 < args.len() => {
                    config.server_name = args[i + 1].clone();
                }
                "--max-connections" if i + 1 < args.len() => {
                    if let Ok(max_conn) = args[i + 1].parse::<u32>() {
                        config.max_connections = max_conn;
                    }
                }
                _ => {}
            }
        }

        eprintln!("⚙️  Configuration loaded:");
        eprintln!("   Server: {} v{}", config.server_name, config.version);
        eprintln!("   Max connections: {}", config.max_connections);
        eprintln!("   Timeout: {}s", config.timeout_seconds);
        eprintln!("   Features: {:?}", config.enabled_features);

        Ok(config)
    }

    // Get enabled tools based on configuration
    pub fn list_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();

        for (tool_name, tool_config) in &self.config.tool_configs {
            if !tool_config.enabled {
                continue;
            }

            let tool = match tool_name.as_str() {
                "greeting" => Tool {
                    name: "greeting".to_string(),
                    description: tool_config.description_override.clone().unwrap_or_else(|| {
                        "Generate personalized greetings in multiple languages".to_string()
                    }),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name of the person to greet"
                            },
                            "language": {
                                "type": "string",
                                "description": "Language for greeting (en, es, fr, de)",
                                "enum": ["en", "es", "fr", "de"],
                                "default": "en"
                            }
                        },
                        "required": ["name"]
                    }),
                },
                "echo" => Tool {
                    name: "echo".to_string(),
                    description: tool_config
                        .description_override
                        .clone()
                        .unwrap_or_else(|| "Echo messages with optional prefix".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "Message to echo back"
                            }
                        },
                        "required": ["message"]
                    }),
                },
                "status" => Tool {
                    name: "status".to_string(),
                    description: tool_config
                        .description_override
                        .clone()
                        .unwrap_or_else(|| "Get server status and statistics".to_string()),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                },
                _ => continue,
            };

            tools.push(tool);
        }

        tools
    }

    // Handle tool calls with configuration support
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        // Increment request counter
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check if tool is enabled
        if let Some(tool_config) = self.config.tool_configs.get(name) {
            if !tool_config.enabled {
                return Err(format!("Tool '{}' is disabled", name));
            }
        } else {
            return Err(format!("Unknown tool: {}", name));
        }

        match name {
            "greeting" => {
                let request: GreetingRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                let greeting = match request.language.as_deref().unwrap_or("en") {
                    "es" => format!(
                        "¡Hola, {}! Bienvenido al servidor MCP configurable.",
                        request.name
                    ),
                    "fr" => format!(
                        "Bonjour, {} ! Bienvenue sur le serveur MCP configurable.",
                        request.name
                    ),
                    "de" => format!(
                        "Hallo, {}! Willkommen beim konfigurierbaren MCP-Server.",
                        request.name
                    ),
                    _ => format!(
                        "Hello, {}! Welcome to the configurable MCP server.",
                        request.name
                    ),
                };

                Ok(serde_json::json!({
                    "message": greeting,
                    "language": request.language.unwrap_or_else(|| "en".to_string()),
                    "server": self.config.server_name
                }))
            }
            "echo" => {
                let request: EchoRequest = serde_json::from_value(arguments)
                    .map_err(|e| format!("Failed to parse arguments: {}", e))?;

                // Get prefix from tool configuration
                let prefix = self
                    .config
                    .tool_configs
                    .get("echo")
                    .and_then(|tc| tc.parameters.get("prefix"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("Echo: ");

                Ok(serde_json::json!({
                    "echo": format!("{}{}", prefix, request.message),
                    "original": request.message,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
            "status" => {
                let uptime = self.start_time.elapsed().as_secs();
                let request_count = self
                    .request_count
                    .load(std::sync::atomic::Ordering::Relaxed);

                let response = StatusResponse {
                    server_name: self.config.server_name.clone(),
                    version: self.config.version.clone(),
                    uptime_seconds: uptime,
                    active_connections: 1, // Simplified for demo
                    enabled_features: self.config.enabled_features.clone(),
                    total_requests: request_count,
                };

                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize status: {}", e))
            }
            _ => Err(format!("Tool implementation not found: {}", name)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("⚙️  Starting Configurable MCP Server");
    eprintln!("=====================================");

    // Load configuration from multiple sources
    let config = ConfigurableServer::load_config()?;

    // Create server with loaded configuration
    let server = ConfigurableServer::new(config);

    // Demo configuration features
    eprintln!("\n🧪 Configuration Demo:");

    // List enabled tools
    let tools = server.list_tools();
    eprintln!("📋 Enabled tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Test greeting in different languages
    eprintln!("\n🌍 Multi-language greeting test:");
    for (lang, name) in [
        ("en", "Developer"),
        ("es", "Desarrollador"),
        ("fr", "Développeur"),
    ] {
        let args = serde_json::json!({
            "name": name,
            "language": lang
        });

        match server.call_tool("greeting", args) {
            Ok(result) => eprintln!(
                "  ✅ {}: {}",
                lang,
                result.get("message").unwrap_or(&Value::Null)
            ),
            Err(e) => eprintln!("  ❌ {}: {}", lang, e),
        }
    }

    // Test echo with configured prefix
    eprintln!("\n📢 Echo test:");
    let echo_args = serde_json::json!({
        "message": "Configuration is working!"
    });

    match server.call_tool("echo", echo_args) {
        Ok(result) => eprintln!("  ✅ {}", result.get("echo").unwrap_or(&Value::Null)),
        Err(e) => eprintln!("  ❌ {}", e),
    }

    // Test status
    eprintln!("\n📊 Server status:");
    match server.call_tool("status", serde_json::json!({})) {
        Ok(result) => {
            if let Ok(status) = serde_json::from_value::<StatusResponse>(result) {
                eprintln!("  ✅ Server: {} v{}", status.server_name, status.version);
                eprintln!("  ⏱️  Uptime: {}s", status.uptime_seconds);
                eprintln!("  📊 Requests: {}", status.total_requests);
                eprintln!("  🔧 Features: {:?}", status.enabled_features);
            }
        }
        Err(e) => eprintln!("  ❌ Status error: {}", e),
    }

    eprintln!("\n🎉 Configuration demo completed!");
    eprintln!("\n💡 Try setting environment variables:");
    eprintln!("   export MCP_SERVER_NAME=\"My Custom Server\"");
    eprintln!("   export MCP_MAX_CONNECTIONS=50");
    eprintln!("   cargo run --bin example_06_configurable_server");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = ServerConfig::default();
        assert_eq!(config.server_name, "Configurable MCP Server");
        assert_eq!(config.max_connections, 100);
        assert!(config.tool_configs.contains_key("greeting"));
        assert!(config.tool_configs.contains_key("echo"));
        assert!(config.tool_configs.contains_key("status"));
    }

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = ConfigurableServer::new(config.clone());

        let tools = server.list_tools();
        assert_eq!(tools.len(), 3); // greeting, echo, status
        assert!(tools.iter().any(|t| t.name == "greeting"));
        assert!(tools.iter().any(|t| t.name == "echo"));
        assert!(tools.iter().any(|t| t.name == "status"));
    }

    #[test]
    fn test_multilingual_greeting() {
        let config = ServerConfig::default();
        let server = ConfigurableServer::new(config);

        // Test English greeting
        let args = serde_json::json!({
            "name": "Test",
            "language": "en"
        });

        let result = server.call_tool("greeting", args).unwrap();
        let message = result.get("message").unwrap().as_str().unwrap();
        assert!(message.contains("Hello, Test"));

        // Test Spanish greeting
        let args = serde_json::json!({
            "name": "Test",
            "language": "es"
        });

        let result = server.call_tool("greeting", args).unwrap();
        let message = result.get("message").unwrap().as_str().unwrap();
        assert!(message.contains("¡Hola, Test"));
    }

    #[test]
    fn test_echo_with_prefix() {
        let config = ServerConfig::default();
        let server = ConfigurableServer::new(config);

        let args = serde_json::json!({
            "message": "test message"
        });

        let result = server.call_tool("echo", args).unwrap();
        let echo = result.get("echo").unwrap().as_str().unwrap();
        assert!(echo.starts_with("Echo: "));
        assert!(echo.contains("test message"));
    }

    #[test]
    fn test_status_tool() {
        let config = ServerConfig::default();
        let server = ConfigurableServer::new(config);

        let result = server.call_tool("status", serde_json::json!({})).unwrap();
        let status: StatusResponse = serde_json::from_value(result).unwrap();

        assert_eq!(status.server_name, "Configurable MCP Server");
        assert_eq!(status.version, "1.0.0");
        // uptime_seconds is u64, so it's always >= 0
    }

    #[test]
    fn test_disabled_tool() {
        let mut config = ServerConfig::default();
        config.tool_configs.get_mut("greeting").unwrap().enabled = false;

        let server = ConfigurableServer::new(config);

        let tools = server.list_tools();
        assert!(!tools.iter().any(|t| t.name == "greeting"));

        let args = serde_json::json!({"name": "Test"});
        let result = server.call_tool("greeting", args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disabled"));
    }
}
