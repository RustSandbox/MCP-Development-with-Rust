// File: src/examples/example_11_monitoring.rs
//
// Example 11: Monitoring and Metrics Server
//
// This example demonstrates building a comprehensive monitoring and metrics collection
// system using MCP. This is a critical component in real-world applications where
// observability and system health monitoring are essential for production systems.
//
// Key Learning Objectives:
// - Health check endpoints and system monitoring
// - Metrics collection and aggregation patterns
// - Performance monitoring and alerting
// - Resource usage tracking
// - Custom metric definitions and collection
// - Time-series data handling
// - Integration with monitoring tools

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// Constants: Define monitoring configuration values as named constants
// This follows clean code principles by avoiding magic numbers
const MAX_METRIC_HISTORY_SIZE: usize = 1000;
const ALERT_THRESHOLD_CPU_PERCENT: f64 = 80.0;
const ALERT_THRESHOLD_MEMORY_PERCENT: f64 = 85.0;

// Struct: SystemMetrics
//
// Represents a snapshot of system metrics at a specific point in time.
// This structure captures key system health indicators that are commonly
// monitored in production environments.
//
// Fields:
//     timestamp: Unix timestamp when metrics were collected
//     cpu_usage_percent: CPU utilization as a percentage (0.0-100.0)
//     memory_usage_percent: Memory utilization as a percentage (0.0-100.0)
//     disk_usage_percent: Disk space utilization as a percentage (0.0-100.0)
//     network_bytes_sent: Total bytes sent over network interfaces
//     network_bytes_received: Total bytes received over network interfaces
//     active_connections: Number of active network connections
//     uptime_seconds: System uptime in seconds
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemMetrics {
    pub timestamp: u64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub active_connections: u32,
    pub uptime_seconds: u64,
}

// Struct: HealthCheckResult
//
// Represents the result of a health check operation.
// Health checks are critical for monitoring system availability and
// detecting issues before they impact users.
//
// Fields:
//     service_name: Name of the service being checked
//     status: Health status ("healthy", "degraded", "unhealthy")
//     response_time_ms: Time taken to perform the health check in milliseconds
//     message: Human-readable status message
//     details: Optional additional details about the health check
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthCheckResult {
    pub service_name: String,
    pub status: String, // "healthy", "degraded", "unhealthy"
    pub response_time_ms: u64,
    pub message: String,
    pub details: Option<Value>,
}

// Struct: Alert
//
// Represents a monitoring alert when system metrics exceed defined thresholds.
// Alerts are essential for proactive monitoring and incident response.
//
// Fields:
//     id: Unique identifier for the alert
//     severity: Alert severity level ("info", "warning", "critical")
//     title: Brief description of the alert
//     description: Detailed description of the issue
//     metric_name: Name of the metric that triggered the alert
//     threshold: The threshold value that was exceeded
//     current_value: The current value of the metric
//     timestamp: When the alert was triggered
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub severity: String, // "info", "warning", "critical"
    pub title: String,
    pub description: String,
    pub metric_name: String,
    pub threshold: f64,
    pub current_value: f64,
    pub timestamp: u64,
}

// Struct: Tool
//
// Represents an MCP tool that can be called by clients.
// This follows the MCP specification for tool definitions.
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Struct: MonitoringServer
//
// The main monitoring server that provides comprehensive system monitoring
// capabilities through MCP tools. This server demonstrates real-world
// monitoring patterns used in production systems.
//
// Fields:
//     name: Server name for identification
//     version: Server version for tracking
//     metrics_history: Thread-safe storage for historical metrics
//     active_alerts: Thread-safe storage for current alerts
//     services_to_monitor: List of services to perform health checks on
//     start_time: Server start time for uptime calculations
pub struct MonitoringServer {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    metrics_history: Arc<Mutex<Vec<SystemMetrics>>>,
    active_alerts: Arc<Mutex<Vec<Alert>>>,
    services_to_monitor: Vec<String>,
    start_time: SystemTime,
}

impl Default for MonitoringServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringServer {
    // Function: new
    //
    // Creates a new monitoring server instance with default configuration.
    // This initializes all the necessary data structures and sets up the
    // monitoring infrastructure.
    //
    // Returns:
    //     A new MonitoringServer instance ready to handle monitoring requests.
    pub fn new() -> Self {
        Self {
            name: "Monitoring and Metrics Server".to_string(),
            version: "1.0.0".to_string(),
            metrics_history: Arc::new(Mutex::new(Vec::new())),
            active_alerts: Arc::new(Mutex::new(Vec::new())),
            services_to_monitor: vec![
                "database".to_string(),
                "web_server".to_string(),
                "cache".to_string(),
                "message_queue".to_string(),
            ],
            start_time: SystemTime::now(),
        }
    }

    // Function: list_tools
    //
    // Returns the list of available monitoring tools that clients can call.
    // Each tool provides specific monitoring capabilities like metrics collection,
    // health checks, alerting, and system status reporting.
    //
    // Returns:
    //     A vector of Tool structs describing all available monitoring tools.
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "get_current_metrics".to_string(),
                description:
                    "Get current system metrics including CPU, memory, disk, and network usage"
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "get_metrics_history".to_string(),
                description: "Get historical metrics data for trend analysis".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of historical records to return",
                            "minimum": 1,
                            "maximum": 1000,
                            "default": 100
                        }
                    },
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "perform_health_check".to_string(),
                description: "Perform health checks on monitored services".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "service_name": {
                            "type": "string",
                            "description": "Specific service to check, or 'all' for all services"
                        }
                    },
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "get_active_alerts".to_string(),
                description: "Get list of current active alerts".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "severity": {
                            "type": "string",
                            "enum": ["info", "warning", "critical"],
                            "description": "Filter alerts by severity level"
                        }
                    },
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "clear_alert".to_string(),
                description: "Clear a specific alert by ID".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "alert_id": {
                            "type": "string",
                            "description": "ID of the alert to clear"
                        }
                    },
                    "required": ["alert_id"],
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "set_alert_threshold".to_string(),
                description: "Configure alert thresholds for metrics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "metric_name": {
                            "type": "string",
                            "description": "Name of the metric to configure"
                        },
                        "threshold": {
                            "type": "number",
                            "description": "Threshold value for triggering alerts"
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["info", "warning", "critical"],
                            "description": "Severity level for alerts triggered by this threshold"
                        }
                    },
                    "required": ["metric_name", "threshold", "severity"],
                    "additionalProperties": false
                }),
            },
        ]
    }

    // Function: call_tool
    //
    // Handles tool calls from MCP clients. This is the main entry point for
    // all monitoring operations. Each tool provides specific monitoring
    // functionality while maintaining consistent error handling and response formatting.
    //
    // Arguments:
    //     name: The name of the tool to call
    //     arguments: JSON arguments specific to each tool
    //
    // Returns:
    //     Result containing the tool response as JSON or an error message
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "get_current_metrics" => {
                // Collect current system metrics
                let metrics = self.collect_current_metrics().await?;

                // Store in history for trend analysis
                self.store_metrics(metrics.clone()).await?;

                // Check for threshold violations and create alerts
                self.check_alert_thresholds(&metrics).await?;

                serde_json::to_value(metrics)
                    .map_err(|e| format!("Failed to serialize metrics: {}", e))
            }
            "get_metrics_history" => {
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as usize;

                let history = self.get_metrics_history(limit).await?;

                serde_json::to_value(serde_json::json!({
                    "total_records": history.len(),
                    "limit": limit,
                    "metrics": history
                }))
                .map_err(|e| format!("Failed to serialize history: {}", e))
            }
            "perform_health_check" => {
                let service_name = arguments
                    .get("service_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all");

                let results = self.perform_health_checks(service_name).await?;

                serde_json::to_value(serde_json::json!({
                    "timestamp": self.get_current_timestamp(),
                    "checks_performed": results.len(),
                    "results": results
                }))
                .map_err(|e| format!("Failed to serialize health check results: {}", e))
            }
            "get_active_alerts" => {
                let severity_filter = arguments.get("severity").and_then(|v| v.as_str());

                let alerts = self.get_active_alerts(severity_filter).await?;

                serde_json::to_value(serde_json::json!({
                    "total_alerts": alerts.len(),
                    "severity_filter": severity_filter,
                    "alerts": alerts
                }))
                .map_err(|e| format!("Failed to serialize alerts: {}", e))
            }
            "clear_alert" => {
                let alert_id = arguments
                    .get("alert_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing required parameter: alert_id")?;

                let cleared = self.clear_alert(alert_id).await?;

                serde_json::to_value(serde_json::json!({
                    "success": cleared,
                    "message": if cleared {
                        format!("Alert {} cleared successfully", alert_id)
                    } else {
                        format!("Alert {} not found", alert_id)
                    }
                }))
                .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            "set_alert_threshold" => {
                let metric_name = arguments
                    .get("metric_name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing required parameter: metric_name")?;

                let threshold = arguments
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .ok_or("Missing required parameter: threshold")?;

                let severity = arguments
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing required parameter: severity")?;

                // In a real implementation, this would store threshold configuration
                // For this demo, we'll just acknowledge the configuration
                serde_json::to_value(serde_json::json!({
                    "success": true,
                    "message": format!("Alert threshold configured for {}", metric_name),
                    "configuration": {
                        "metric_name": metric_name,
                        "threshold": threshold,
                        "severity": severity
                    }
                }))
                .map_err(|e| format!("Failed to serialize response: {}", e))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // Function: collect_current_metrics
    //
    // Simulates collection of current system metrics.
    // In a real implementation, this would interface with system APIs
    // to gather actual performance data.
    //
    // Returns:
    //     Result containing current SystemMetrics or an error
    async fn collect_current_metrics(&self) -> Result<SystemMetrics, String> {
        // Simulate metric collection with realistic but randomized values
        // In production, this would query actual system resources

        let timestamp = self.get_current_timestamp();
        let uptime = self
            .start_time
            .elapsed()
            .map_err(|e| format!("Failed to calculate uptime: {}", e))?
            .as_secs();

        // Generate realistic but simulated metrics
        // In production, these would come from system monitoring APIs
        let metrics = SystemMetrics {
            timestamp,
            cpu_usage_percent: 20.0 + (timestamp % 60) as f64 * 0.8, // Varies between 20-68%
            memory_usage_percent: 45.0 + (timestamp % 40) as f64 * 0.5, // Varies between 45-65%
            disk_usage_percent: 35.0 + (timestamp % 10) as f64 * 0.2, // Varies between 35-37%
            network_bytes_sent: 1024 * 1024 * (timestamp % 1000),    // Simulated network activity
            network_bytes_received: 2 * 1024 * 1024 * (timestamp % 1000),
            active_connections: 50 + (timestamp % 100) as u32, // 50-149 connections
            uptime_seconds: uptime,
        };

        Ok(metrics)
    }

    // Function: store_metrics
    //
    // Stores metrics in the historical data collection with size management.
    // This implements a circular buffer pattern to prevent unbounded memory growth.
    //
    // Arguments:
    //     metrics: SystemMetrics to store in history
    //
    // Returns:
    //     Result indicating success or failure
    async fn store_metrics(&self, metrics: SystemMetrics) -> Result<(), String> {
        let mut history = self
            .metrics_history
            .lock()
            .map_err(|e| format!("Failed to acquire metrics history lock: {}", e))?;

        // Add new metrics to history
        history.push(metrics);

        // Implement circular buffer: remove oldest entries if we exceed maximum size
        // This prevents unbounded memory growth in long-running systems
        if history.len() > MAX_METRIC_HISTORY_SIZE {
            let excess = history.len() - MAX_METRIC_HISTORY_SIZE;
            history.drain(0..excess);
        }

        Ok(())
    }

    // Function: get_metrics_history
    //
    // Retrieves historical metrics data for trend analysis and reporting.
    //
    // Arguments:
    //     limit: Maximum number of records to return
    //
    // Returns:
    //     Result containing vector of historical SystemMetrics
    async fn get_metrics_history(&self, limit: usize) -> Result<Vec<SystemMetrics>, String> {
        let history = self
            .metrics_history
            .lock()
            .map_err(|e| format!("Failed to acquire metrics history lock: {}", e))?;

        // Return the most recent 'limit' entries
        let start_index = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };

        Ok(history[start_index..].to_vec())
    }

    // Function: check_alert_thresholds
    //
    // Checks current metrics against predefined thresholds and creates alerts
    // when thresholds are exceeded. This is critical for proactive monitoring.
    //
    // Arguments:
    //     metrics: Current SystemMetrics to check against thresholds
    //
    // Returns:
    //     Result indicating success or failure of threshold checking
    async fn check_alert_thresholds(&self, metrics: &SystemMetrics) -> Result<(), String> {
        let mut alerts = self
            .active_alerts
            .lock()
            .map_err(|e| format!("Failed to acquire alerts lock: {}", e))?;

        // Check CPU usage threshold
        if metrics.cpu_usage_percent > ALERT_THRESHOLD_CPU_PERCENT {
            let alert = Alert {
                id: format!("cpu-{}", metrics.timestamp),
                severity: "warning".to_string(),
                title: "High CPU Usage".to_string(),
                description: format!(
                    "CPU usage is {}%, exceeding threshold of {}%",
                    metrics.cpu_usage_percent, ALERT_THRESHOLD_CPU_PERCENT
                ),
                metric_name: "cpu_usage_percent".to_string(),
                threshold: ALERT_THRESHOLD_CPU_PERCENT,
                current_value: metrics.cpu_usage_percent,
                timestamp: metrics.timestamp,
            };
            alerts.push(alert);
        }

        // Check memory usage threshold
        if metrics.memory_usage_percent > ALERT_THRESHOLD_MEMORY_PERCENT {
            let alert = Alert {
                id: format!("memory-{}", metrics.timestamp),
                severity: "critical".to_string(),
                title: "High Memory Usage".to_string(),
                description: format!(
                    "Memory usage is {}%, exceeding threshold of {}%",
                    metrics.memory_usage_percent, ALERT_THRESHOLD_MEMORY_PERCENT
                ),
                metric_name: "memory_usage_percent".to_string(),
                threshold: ALERT_THRESHOLD_MEMORY_PERCENT,
                current_value: metrics.memory_usage_percent,
                timestamp: metrics.timestamp,
            };
            alerts.push(alert);
        }

        Ok(())
    }

    // Function: perform_health_checks
    //
    // Performs health checks on monitored services to ensure they are
    // functioning correctly. This is essential for service availability monitoring.
    //
    // Arguments:
    //     service_filter: Either "all" or a specific service name to check
    //
    // Returns:
    //     Result containing vector of HealthCheckResult
    async fn perform_health_checks(
        &self,
        service_filter: &str,
    ) -> Result<Vec<HealthCheckResult>, String> {
        let mut results = Vec::new();

        let services_to_check: Vec<String> = if service_filter == "all" {
            self.services_to_monitor.clone()
        } else {
            // Check if the specific service is in our monitoring list
            if self
                .services_to_monitor
                .contains(&service_filter.to_string())
            {
                vec![service_filter.to_string()]
            } else {
                return Err(format!(
                    "Service '{}' is not being monitored",
                    service_filter
                ));
            }
        };

        for service_name in &services_to_check {
            let start_time = std::time::Instant::now();

            // Simulate health check operation
            // In production, this would make actual HTTP requests or service calls
            sleep(Duration::from_millis(10)).await; // Simulate check latency

            let response_time = start_time.elapsed().as_millis() as u64;

            // Simulate different health states based on service name
            let (status, message) = match service_name.as_str() {
                "database" => ("healthy", "Database connection pool is responding normally"),
                "web_server" => ("healthy", "Web server is accepting requests"),
                "cache" => ("degraded", "Cache hit ratio is below optimal threshold"),
                "message_queue" => ("healthy", "Message queue is processing messages normally"),
                _ => ("unknown", "Service status could not be determined"),
            };

            let result = HealthCheckResult {
                service_name: service_name.clone(),
                status: status.to_string(),
                response_time_ms: response_time,
                message: message.to_string(),
                details: Some(serde_json::json!({
                    "checked_at": self.get_current_timestamp(),
                    "check_type": "synthetic",
                    "endpoint": format!("internal://{}/health", service_name)
                })),
            };

            results.push(result);
        }

        Ok(results)
    }

    // Function: get_active_alerts
    //
    // Retrieves current active alerts, optionally filtered by severity level.
    //
    // Arguments:
    //     severity_filter: Optional severity level to filter alerts
    //
    // Returns:
    //     Result containing vector of filtered Alert objects
    async fn get_active_alerts(&self, severity_filter: Option<&str>) -> Result<Vec<Alert>, String> {
        let alerts = self
            .active_alerts
            .lock()
            .map_err(|e| format!("Failed to acquire alerts lock: {}", e))?;

        let filtered_alerts = if let Some(severity) = severity_filter {
            alerts
                .iter()
                .filter(|alert| alert.severity == severity)
                .cloned()
                .collect()
        } else {
            alerts.clone()
        };

        Ok(filtered_alerts)
    }

    // Function: clear_alert
    //
    // Clears (removes) a specific alert by its ID.
    // This is used for alert acknowledgment and resolution.
    //
    // Arguments:
    //     alert_id: The ID of the alert to clear
    //
    // Returns:
    //     Result indicating whether the alert was found and cleared
    async fn clear_alert(&self, alert_id: &str) -> Result<bool, String> {
        let mut alerts = self
            .active_alerts
            .lock()
            .map_err(|e| format!("Failed to acquire alerts lock: {}", e))?;

        let initial_len = alerts.len();
        alerts.retain(|alert| alert.id != alert_id);
        let cleared = alerts.len() < initial_len;

        Ok(cleared)
    }

    // Function: get_current_timestamp
    //
    // Gets the current Unix timestamp for consistent time tracking.
    //
    // Returns:
    //     Current Unix timestamp in seconds
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

// Function: main
//
// The main entry point that demonstrates the monitoring server capabilities.
// This showcases various monitoring operations and how they would be used
// in a real-world monitoring system.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for observability
    tracing_subscriber::fmt::init();

    eprintln!("🚀 Starting Monitoring and Metrics Server");
    eprintln!("==========================================");

    let server = MonitoringServer::new();

    eprintln!("\n🧪 Monitoring and Metrics Demo:");

    // List available tools
    let tools = server.list_tools();
    eprintln!("📋 Available monitoring tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Demonstrate metrics collection
    eprintln!("\n📊 Collecting current system metrics:");
    match server
        .call_tool("get_current_metrics", serde_json::json!({}))
        .await
    {
        Ok(result) => {
            let metrics: SystemMetrics = serde_json::from_value(result).unwrap();
            eprintln!("  ✅ Metrics collected successfully");
            eprintln!("     CPU Usage: {:.1}%", metrics.cpu_usage_percent);
            eprintln!("     Memory Usage: {:.1}%", metrics.memory_usage_percent);
            eprintln!("     Disk Usage: {:.1}%", metrics.disk_usage_percent);
            eprintln!("     Active Connections: {}", metrics.active_connections);
            eprintln!("     Uptime: {} seconds", metrics.uptime_seconds);
        }
        Err(e) => eprintln!("  ❌ Metrics collection failed: {}", e),
    }

    // Wait a moment and collect metrics again to build history
    sleep(Duration::from_millis(100)).await;
    let _ = server
        .call_tool("get_current_metrics", serde_json::json!({}))
        .await;

    // Demonstrate metrics history
    eprintln!("\n📈 Retrieving metrics history:");
    match server
        .call_tool("get_metrics_history", serde_json::json!({"limit": 5}))
        .await
    {
        Ok(result) => {
            let history_data: Value = result;
            let total_records = history_data.get("total_records").unwrap_or(&Value::Null);
            eprintln!("  ✅ Retrieved metrics history");
            eprintln!("     Total records: {}", total_records);
        }
        Err(e) => eprintln!("  ❌ History retrieval failed: {}", e),
    }

    // Demonstrate health checks
    eprintln!("\n🏥 Performing health checks:");
    match server
        .call_tool(
            "perform_health_check",
            serde_json::json!({"service_name": "all"}),
        )
        .await
    {
        Ok(result) => {
            let health_data: Value = result;
            let checks_performed = health_data.get("checks_performed").unwrap_or(&Value::Null);
            eprintln!("  ✅ Health checks completed");
            eprintln!("     Checks performed: {}", checks_performed);

            if let Some(results) = health_data.get("results") {
                if let Some(results_array) = results.as_array() {
                    for result in results_array {
                        if let Ok(check) =
                            serde_json::from_value::<HealthCheckResult>(result.clone())
                        {
                            eprintln!(
                                "     - {}: {} ({}ms)",
                                check.service_name, check.status, check.response_time_ms
                            );
                        }
                    }
                }
            }
        }
        Err(e) => eprintln!("  ❌ Health checks failed: {}", e),
    }

    // Demonstrate alert management
    eprintln!("\n🚨 Checking active alerts:");
    match server
        .call_tool("get_active_alerts", serde_json::json!({}))
        .await
    {
        Ok(result) => {
            let alerts_data: Value = result;
            let total_alerts = alerts_data.get("total_alerts").unwrap_or(&Value::Null);
            eprintln!("  ✅ Alert check completed");
            eprintln!("     Active alerts: {}", total_alerts);
        }
        Err(e) => eprintln!("  ❌ Alert check failed: {}", e),
    }

    // Demonstrate threshold configuration
    eprintln!("\n⚙️  Configuring alert thresholds:");
    let threshold_config = serde_json::json!({
        "metric_name": "cpu_usage_percent",
        "threshold": 75.0,
        "severity": "warning"
    });

    match server
        .call_tool("set_alert_threshold", threshold_config)
        .await
    {
        Ok(result) => {
            let config_data: Value = result;
            let success = config_data.get("success").unwrap_or(&Value::Null);
            eprintln!("  ✅ Threshold configuration: {}", success);
        }
        Err(e) => eprintln!("  ❌ Threshold configuration failed: {}", e),
    }

    eprintln!("\n🎉 Monitoring and Metrics demo completed!");
    eprintln!("\n✨ This is example 11 of 20 progressive MCP examples.");
    eprintln!("   This example demonstrates comprehensive monitoring patterns");
    eprintln!("   essential for production systems including:");
    eprintln!("   - Real-time metrics collection and storage");
    eprintln!("   - Health check orchestration and reporting");
    eprintln!("   - Threshold-based alerting and notification");
    eprintln!("   - Historical data management and trend analysis");
    eprintln!("   - Configurable monitoring parameters");
    eprintln!("\n🔧 Key production monitoring concepts covered:");
    eprintln!("   - Circular buffer for metrics history management");
    eprintln!("   - Thread-safe data structures for concurrent access");
    eprintln!("   - Realistic simulation of system resource monitoring");
    eprintln!("   - Alert lifecycle management (creation, filtering, clearing)");
    eprintln!("   - Extensible architecture for additional monitoring tools");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_server_creation() {
        let server = MonitoringServer::new();
        assert_eq!(server.name, "Monitoring and Metrics Server");
        assert_eq!(server.version, "1.0.0");
        assert_eq!(server.services_to_monitor.len(), 4);
    }

    #[tokio::test]
    async fn test_tools_listing() {
        let server = MonitoringServer::new();
        let tools = server.list_tools();

        assert_eq!(tools.len(), 6);
        assert!(tools.iter().any(|t| t.name == "get_current_metrics"));
        assert!(tools.iter().any(|t| t.name == "get_metrics_history"));
        assert!(tools.iter().any(|t| t.name == "perform_health_check"));
        assert!(tools.iter().any(|t| t.name == "get_active_alerts"));
        assert!(tools.iter().any(|t| t.name == "clear_alert"));
        assert!(tools.iter().any(|t| t.name == "set_alert_threshold"));
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let server = MonitoringServer::new();
        let result = server
            .call_tool("get_current_metrics", serde_json::json!({}))
            .await;

        assert!(result.is_ok());
        let metrics: SystemMetrics = serde_json::from_value(result.unwrap()).unwrap();
        assert!(metrics.cpu_usage_percent >= 0.0 && metrics.cpu_usage_percent <= 100.0);
        assert!(metrics.memory_usage_percent >= 0.0 && metrics.memory_usage_percent <= 100.0);
        // uptime_seconds might be 0 in fast test environments
        assert!(metrics.uptime_seconds < 1000); // Just verify it's a reasonable value
    }

    #[tokio::test]
    async fn test_health_checks() {
        let server = MonitoringServer::new();
        let result = server
            .call_tool(
                "perform_health_check",
                serde_json::json!({"service_name": "database"}),
            )
            .await;

        assert!(result.is_ok());
        let health_data: Value = result.unwrap();
        assert_eq!(health_data.get("checks_performed").unwrap(), 1);
    }

    #[tokio::test]
    async fn test_alert_management() {
        let server = MonitoringServer::new();

        // Test getting alerts (should be empty initially)
        let result = server
            .call_tool("get_active_alerts", serde_json::json!({}))
            .await;
        assert!(result.is_ok());

        let alerts_data: Value = result.unwrap();
        assert_eq!(alerts_data.get("total_alerts").unwrap(), 0);
    }

    #[tokio::test]
    async fn test_threshold_configuration() {
        let server = MonitoringServer::new();
        let threshold_config = serde_json::json!({
            "metric_name": "test_metric",
            "threshold": 80.0,
            "severity": "warning"
        });

        let result = server
            .call_tool("set_alert_threshold", threshold_config)
            .await;
        assert!(result.is_ok());

        let config_data: Value = result.unwrap();
        assert_eq!(config_data.get("success").unwrap(), true);
    }
}
