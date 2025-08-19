// File: src/examples/example_10_streaming.rs
//
// This example demonstrates real-time streaming capabilities in an MCP server.
// It shows how to handle live data feeds, async channels, and streaming responses
// for real-time applications.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};

// Streaming configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamingConfig {
    pub max_subscribers: usize,
    pub buffer_size: usize,
    pub heartbeat_interval_ms: u64,
    pub data_generation_interval_ms: u64,
    pub enable_metrics: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_subscribers: 100,
            buffer_size: 1000,
            heartbeat_interval_ms: 5000,
            data_generation_interval_ms: 1000,
            enable_metrics: true,
        }
    }
}

// Message types for streaming
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamMessage {
    pub id: u64,
    pub message_type: String,
    pub data: Value,
    pub timestamp: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricsData {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: u32,
    pub messages_sent: u64,
    pub uptime_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub component: String,
    pub timestamp: String,
}

// Request structures
#[derive(Serialize, Deserialize, Debug)]
pub struct StartStreamRequest {
    pub stream_type: String,
    pub frequency_ms: Option<u64>,
    pub duration_seconds: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetStreamStatsRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendCustomMessageRequest {
    pub message: String,
    pub data: Option<Value>,
}

// Response structures
#[derive(Serialize, Deserialize, Debug)]
pub struct StreamStats {
    pub active_streams: u32,
    pub total_messages: u64,
    pub subscriber_count: usize,
    pub buffer_utilization: f64,
    pub uptime_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Streaming Server
pub struct StreamingServer {
    config: StreamingConfig,
    broadcast_tx: broadcast::Sender<StreamMessage>,
    message_counter: Arc<AtomicU64>,
    start_time: Instant,
}

impl StreamingServer {
    pub fn new(config: StreamingConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config.buffer_size);

        Self {
            config,
            broadcast_tx,
            message_counter: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    // Start background data generation
    pub fn start_background_streams(&self) {
        let tx = self.broadcast_tx.clone();
        let counter = self.message_counter.clone();
        let interval = self.config.data_generation_interval_ms;

        // Spawn metrics stream
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval));

            loop {
                interval.tick().await;

                let id = counter.fetch_add(1, Ordering::Relaxed);
                let metrics = MetricsData {
                    cpu_usage: rand::random::<f64>() * 100.0,
                    memory_usage: rand::random::<f64>() * 100.0,
                    active_connections: rand::random::<u8>() as u32,
                    messages_sent: id,
                    uptime_seconds: id / 10, // Simulated uptime
                };

                let message = StreamMessage {
                    id,
                    message_type: "metrics".to_string(),
                    data: serde_json::to_value(&metrics).unwrap_or_default(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    source: "metrics_generator".to_string(),
                };

                let _ = tx.send(message);
            }
        });

        // Spawn log stream
        let tx = self.broadcast_tx.clone();
        let counter = self.message_counter.clone();
        let log_interval = interval * 2; // Less frequent logs

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(log_interval));
            let log_levels = ["INFO", "WARN", "ERROR", "DEBUG"];
            let components = ["auth", "api", "database", "cache"];
            let messages = [
                "Request processed successfully",
                "Cache miss, fetching from database",
                "Rate limit exceeded for user",
                "Connection pool exhausted",
                "Health check passed",
            ];

            loop {
                interval.tick().await;

                let id = counter.fetch_add(1, Ordering::Relaxed);
                let log_entry = LogEntry {
                    level: log_levels[rand::random::<usize>() % log_levels.len()].to_string(),
                    message: messages[rand::random::<usize>() % messages.len()].to_string(),
                    component: components[rand::random::<usize>() % components.len()].to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                let message = StreamMessage {
                    id,
                    message_type: "log".to_string(),
                    data: serde_json::to_value(&log_entry).unwrap_or_default(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    source: "log_generator".to_string(),
                };

                let _ = tx.send(message);
            }
        });
    }

    // Get recent messages from the stream
    pub async fn get_recent_messages(
        &self,
        count: usize,
        message_type: Option<String>,
    ) -> Vec<StreamMessage> {
        let mut rx = self.broadcast_tx.subscribe();
        let mut messages = Vec::new();
        let timeout = Duration::from_millis(100);

        // Collect recent messages with timeout
        let deadline = Instant::now() + timeout;

        while messages.len() < count && Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(10), rx.recv()).await {
                Ok(Ok(message)) => {
                    if let Some(ref filter_type) = message_type {
                        if message.message_type == *filter_type {
                            messages.push(message);
                        }
                    } else {
                        messages.push(message);
                    }
                }
                _ => break,
            }
        }

        messages
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "start_stream".to_string(),
                description: "Start a real-time data stream".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "stream_type": {
                            "type": "string",
                            "description": "Type of stream to start",
                            "enum": ["metrics", "logs", "events", "all"]
                        },
                        "frequency_ms": {
                            "type": "integer",
                            "description": "Message frequency in milliseconds",
                            "default": 1000,
                            "minimum": 100
                        },
                        "duration_seconds": {
                            "type": "integer",
                            "description": "Stream duration in seconds (0 for unlimited)",
                            "default": 30
                        }
                    },
                    "required": ["stream_type"]
                }),
            },
            Tool {
                name: "get_stream_stats".to_string(),
                description: "Get streaming server statistics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "get_recent_messages".to_string(),
                description: "Get recent messages from the stream".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of recent messages to retrieve",
                            "default": 10,
                            "maximum": 100
                        },
                        "message_type": {
                            "type": "string",
                            "description": "Filter by message type (optional)",
                            "enum": ["metrics", "logs", "events"]
                        }
                    }
                }),
            },
            Tool {
                name: "send_custom_message".to_string(),
                description: "Send a custom message to all subscribers".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Custom message to broadcast"
                        },
                        "data": {
                            "type": "object",
                            "description": "Additional data to include (optional)"
                        }
                    },
                    "required": ["message"]
                }),
            },
        ]
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "start_stream" => self.start_stream(arguments).await,
            "get_stream_stats" => self.get_stream_stats(arguments).await,
            "get_recent_messages" => self.get_recent_messages_tool(arguments).await,
            "send_custom_message" => self.send_custom_message(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn start_stream(&self, arguments: Value) -> Result<Value, String> {
        let request: StartStreamRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let duration = request.duration_seconds.unwrap_or(30);
        let stream_type = request.stream_type.clone();
        let stream_type_for_message = request.stream_type.clone();

        // Start a temporary stream for the specified duration
        let tx = self.broadcast_tx.clone();
        let counter = self.message_counter.clone();
        let frequency = request.frequency_ms.unwrap_or(1000);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(frequency));
            let start = Instant::now();
            let duration = Duration::from_secs(duration);

            while start.elapsed() < duration {
                interval.tick().await;

                let id = counter.fetch_add(1, Ordering::Relaxed);
                let data = match stream_type.as_str() {
                    "metrics" => serde_json::json!({
                        "cpu": rand::random::<f64>() * 100.0,
                        "memory": rand::random::<f64>() * 100.0,
                        "network": rand::random::<f64>() * 1000.0
                    }),
                    "logs" => serde_json::json!({
                        "level": "INFO",
                        "message": "Streaming test message",
                        "request_id": format!("req_{}", id)
                    }),
                    "events" => serde_json::json!({
                        "event_type": "user_action",
                        "user_id": rand::random::<u32>(),
                        "action": "page_view"
                    }),
                    _ => serde_json::json!({
                        "type": "generic",
                        "value": rand::random::<f64>()
                    }),
                };

                let message = StreamMessage {
                    id,
                    message_type: stream_type.clone(),
                    data,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    source: "streaming_tool".to_string(),
                };

                let _ = tx.send(message);
            }
        });

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Started {} stream for {} seconds", stream_type_for_message, duration),
            "stream_type": stream_type_for_message,
            "duration_seconds": duration,
            "frequency_ms": frequency
        }))
    }

    async fn get_stream_stats(&self, _arguments: Value) -> Result<Value, String> {
        let stats = StreamStats {
            active_streams: 2, // Background streams
            total_messages: self.message_counter.load(Ordering::Relaxed),
            subscriber_count: self.broadcast_tx.receiver_count(),
            buffer_utilization: (self.broadcast_tx.len() as f64 / self.config.buffer_size as f64)
                * 100.0,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        };

        serde_json::to_value(stats).map_err(|e| format!("Failed to serialize stats: {}", e))
    }

    async fn get_recent_messages_tool(&self, arguments: Value) -> Result<Value, String> {
        let count = arguments
            .get("count")
            .and_then(|c| c.as_u64())
            .unwrap_or(10) as usize;

        let message_type = arguments
            .get("message_type")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let messages = self.get_recent_messages(count, message_type).await;

        Ok(serde_json::json!({
            "messages": messages,
            "count": messages.len(),
            "requested_count": count
        }))
    }

    async fn send_custom_message(&self, arguments: Value) -> Result<Value, String> {
        let request: SendCustomMessageRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let id = self.message_counter.fetch_add(1, Ordering::Relaxed);
        let message = StreamMessage {
            id,
            message_type: "custom".to_string(),
            data: serde_json::json!({
                "message": request.message,
                "custom_data": request.data.unwrap_or_default()
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: "user".to_string(),
        };

        match self.broadcast_tx.send(message.clone()) {
            Ok(subscriber_count) => Ok(serde_json::json!({
                "success": true,
                "message_id": id,
                "subscriber_count": subscriber_count,
                "sent_message": message
            })),
            Err(_) => Err("Failed to send message (no active subscribers)".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("📡 Starting Real-time Streaming MCP Server");
    eprintln!("==========================================");

    // Create config
    let config = StreamingConfig::default();

    eprintln!("⚙️  Streaming Configuration:");
    eprintln!("   Max subscribers: {}", config.max_subscribers);
    eprintln!("   Buffer size: {}", config.buffer_size);
    eprintln!("   Data interval: {}ms", config.data_generation_interval_ms);
    eprintln!("   Heartbeat interval: {}ms", config.heartbeat_interval_ms);

    // Create server
    let server = StreamingServer::new(config);

    // Start background streams
    server.start_background_streams();

    eprintln!("\n🧪 Streaming Demo:");

    // List tools
    let tools = server.list_tools();
    eprintln!("📋 Available tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Wait a moment for some data to be generated
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Get stream stats
    eprintln!("\n📊 Stream statistics:");
    match server
        .call_tool("get_stream_stats", serde_json::json!({}))
        .await
    {
        Ok(result) => {
            if let Ok(stats) = serde_json::from_value::<StreamStats>(result) {
                eprintln!("  ✅ Active streams: {}", stats.active_streams);
                eprintln!("     Total messages: {}", stats.total_messages);
                eprintln!("     Subscribers: {}", stats.subscriber_count);
                eprintln!("     Buffer utilization: {:.1}%", stats.buffer_utilization);
                eprintln!("     Uptime: {}s", stats.uptime_seconds);
            }
        }
        Err(e) => eprintln!("  ❌ Stats failed: {}", e),
    }

    // Get recent messages
    eprintln!("\n📨 Recent messages:");
    match server
        .call_tool(
            "get_recent_messages",
            serde_json::json!({
                "count": 3,
                "message_type": "metrics"
            }),
        )
        .await
    {
        Ok(result) => {
            let default_count = Value::Number(serde_json::Number::from(0));
            let count = result.get("count").unwrap_or(&default_count);
            eprintln!("  ✅ Retrieved {} recent metrics messages", count);

            if let Some(messages) = result.get("messages").and_then(|m| m.as_array()) {
                for message in messages.iter().take(2) {
                    if let Some(id) = message.get("id") {
                        eprintln!(
                            "     Message {}: {}",
                            id,
                            message.get("message_type").unwrap_or(&Value::Null)
                        );
                    }
                }
            }
        }
        Err(e) => eprintln!("  ❌ Get messages failed: {}", e),
    }

    // Send custom message
    eprintln!("\n📤 Sending custom message:");
    match server
        .call_tool(
            "send_custom_message",
            serde_json::json!({
                "message": "Demo streaming server is working!",
                "data": {"demo": true, "timestamp": chrono::Utc::now().to_rfc3339()}
            }),
        )
        .await
    {
        Ok(result) => {
            let message_id = result.get("message_id").unwrap_or(&Value::Null);
            let subscriber_count = result.get("subscriber_count").unwrap_or(&Value::Null);
            eprintln!(
                "  ✅ Sent message {} to {} subscribers",
                message_id, subscriber_count
            );
        }
        Err(e) => eprintln!("  ❌ Send message failed: {}", e),
    }

    // Start a temporary stream
    eprintln!("\n🎬 Starting demo stream:");
    match server
        .call_tool(
            "start_stream",
            serde_json::json!({
                "stream_type": "events",
                "frequency_ms": 500,
                "duration_seconds": 3
            }),
        )
        .await
    {
        Ok(result) => {
            eprintln!(
                "  ✅ Started event stream: {}",
                result.get("message").unwrap_or(&Value::Null)
            );

            // Wait for the stream to run
            tokio::time::sleep(Duration::from_millis(4000)).await;

            // Check final stats
            if let Ok(final_stats) = server
                .call_tool("get_stream_stats", serde_json::json!({}))
                .await
            {
                if let Ok(stats) = serde_json::from_value::<StreamStats>(final_stats) {
                    eprintln!("  📈 Final message count: {}", stats.total_messages);
                }
            }
        }
        Err(e) => eprintln!("  ❌ Start stream failed: {}", e),
    }

    eprintln!("\n🎉 Streaming demo completed!");
    eprintln!("\n🌊 Streaming features demonstrated:");
    eprintln!("   ✅ Real-time message broadcasting");
    eprintln!("   ✅ Multiple concurrent streams");
    eprintln!("   ✅ Async channel-based communication");
    eprintln!("   ✅ Subscriber management");
    eprintln!("   ✅ Message filtering and retrieval");
    eprintln!("   ✅ Stream statistics and monitoring");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_server() {
        let config = StreamingConfig::default();
        let server = StreamingServer::new(config);

        let tools = server.list_tools();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().any(|t| t.name == "start_stream"));
        assert!(tools.iter().any(|t| t.name == "get_stream_stats"));
        assert!(tools.iter().any(|t| t.name == "send_custom_message"));
    }

    #[tokio::test]
    async fn test_stream_stats() {
        let config = StreamingConfig::default();
        let server = StreamingServer::new(config);

        let result = server
            .call_tool("get_stream_stats", serde_json::json!({}))
            .await
            .unwrap();
        let stats: StreamStats = serde_json::from_value(result).unwrap();

        assert_eq!(stats.active_streams, 2);
        assert_eq!(stats.subscriber_count, 0); // No subscribers in test
    }

    #[tokio::test]
    async fn test_custom_message() {
        let config = StreamingConfig::default();
        let server = StreamingServer::new(config);

        let args = serde_json::json!({
            "message": "Test message",
            "data": {"test": true}
        });

        // This will fail with no subscribers, which is expected
        let result = server.call_tool("send_custom_message", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no active subscribers"));
    }
}
