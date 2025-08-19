// File: src/examples/example_07_file_operations.rs
//
// This example demonstrates safe file system operations in an MCP server.
// It includes security controls, path validation, and various file operations
// while maintaining safety and preventing unauthorized access.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

// Configuration for file operations with security settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileOperationsConfig {
    pub allowed_directories: Vec<PathBuf>,
    pub max_file_size: u64,
    pub allowed_extensions: Vec<String>,
    pub read_only_mode: bool,
    pub enable_directory_listing: bool,
}

impl Default for FileOperationsConfig {
    fn default() -> Self {
        Self {
            allowed_directories: vec![
                PathBuf::from("./temp"),
                PathBuf::from("./data"),
                PathBuf::from("./examples"),
            ],
            max_file_size: 1024 * 1024, // 1MB
            allowed_extensions: vec![
                ".txt".to_string(),
                ".json".to_string(),
                ".md".to_string(),
                ".csv".to_string(),
                ".log".to_string(),
            ],
            read_only_mode: false,
            enable_directory_listing: true,
        }
    }
}

// Request and response structures
#[derive(Serialize, Deserialize, Debug)]
pub struct ReadFileRequest {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WriteFileRequest {
    pub file_path: String,
    pub content: String,
    pub create_directories: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListDirectoryRequest {
    pub directory_path: String,
    pub include_hidden: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteFileRequest {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: u64,
    pub modified: String,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DirectoryListing {
    pub path: String,
    pub files: Vec<FileInfo>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// Custom error types for file operations
#[derive(Debug)]
pub enum FileOperationError {
    SecurityViolation(String),
    InvalidPath(String),
    PermissionDenied(String),
    FileNotFound(String),
    FileTooLarge(String),
    UnsupportedExtension(String),
    IoError(String),
}

impl std::fmt::Display for FileOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperationError::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            FileOperationError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            FileOperationError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            FileOperationError::FileNotFound(msg) => write!(f, "File not found: {}", msg),
            FileOperationError::FileTooLarge(msg) => write!(f, "File too large: {}", msg),
            FileOperationError::UnsupportedExtension(msg) => {
                write!(f, "Unsupported extension: {}", msg)
            }
            FileOperationError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for FileOperationError {}

// File Operations Server
pub struct FileOperationsServer {
    config: FileOperationsConfig,
}

impl FileOperationsServer {
    pub fn new(config: FileOperationsConfig) -> Self {
        Self { config }
    }

    // Validate that a path is safe and allowed
    fn validate_path(&self, path: &str) -> Result<PathBuf, FileOperationError> {
        let path = Path::new(path);

        // Convert to absolute path to prevent directory traversal
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If canonicalize fails, the file might not exist yet
                // Try to canonicalize the parent directory
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        let canonical_parent = parent
                            .canonicalize()
                            .map_err(|e| FileOperationError::InvalidPath(e.to_string()))?;
                        canonical_parent.join(path.file_name().unwrap_or_default())
                    } else {
                        return Err(FileOperationError::InvalidPath(
                            "Parent directory does not exist".to_string(),
                        ));
                    }
                } else {
                    return Err(FileOperationError::InvalidPath(
                        "Invalid path structure".to_string(),
                    ));
                }
            }
        };

        // Check if path is within allowed directories
        let mut allowed = false;
        for allowed_dir in &self.config.allowed_directories {
            if let Ok(canonical_allowed) = allowed_dir.canonicalize() {
                if canonical_path.starts_with(&canonical_allowed) {
                    allowed = true;
                    break;
                }
            }
        }

        if !allowed {
            return Err(FileOperationError::SecurityViolation(format!(
                "Path '{}' is not in an allowed directory",
                canonical_path.display()
            )));
        }

        // Check file extension if it exists
        if let Some(extension) = canonical_path.extension() {
            let ext = format!(".{}", extension.to_string_lossy().to_lowercase());
            if !self.config.allowed_extensions.contains(&ext) {
                return Err(FileOperationError::UnsupportedExtension(format!(
                    "Extension '{}' is not allowed",
                    ext
                )));
            }
        }

        Ok(canonical_path)
    }

    // Check file size constraints
    fn validate_file_size(&self, size: u64) -> Result<(), FileOperationError> {
        if size > self.config.max_file_size {
            return Err(FileOperationError::FileTooLarge(format!(
                "File size {} bytes exceeds maximum of {} bytes",
                size, self.config.max_file_size
            )));
        }
        Ok(())
    }

    // Create FileInfo from a path
    async fn create_file_info(&self, path: &Path) -> Result<FileInfo, FileOperationError> {
        let metadata = async_fs::metadata(path)
            .await
            .map_err(|e| FileOperationError::IoError(e.to_string()))?;

        let file_type = if metadata.is_dir() {
            "directory".to_string()
        } else if metadata.is_file() {
            "file".to_string()
        } else {
            "other".to_string()
        };

        let modified = match metadata.modified() {
            Ok(time) => match time.duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
                    .unwrap_or_default()
                    .to_rfc3339(),
                Err(_) => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
        };

        Ok(FileInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            file_type,
            size: metadata.len(),
            modified,
            readable: true, // Simplified for demo
            writable: !self.config.read_only_mode,
        })
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool {
                name: "read_file".to_string(),
                description: "Read the contents of a text file safely".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["file_path"]
                }),
            },
            Tool {
                name: "get_file_info".to_string(),
                description: "Get information about a file or directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file or directory"
                        }
                    },
                    "required": ["file_path"]
                }),
            },
        ];

        if !self.config.read_only_mode {
            tools.extend([
                Tool {
                    name: "write_file".to_string(),
                    description: "Write content to a file safely".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write to the file"
                            },
                            "create_directories": {
                                "type": "boolean",
                                "description": "Whether to create parent directories if they don't exist",
                                "default": false
                            }
                        },
                        "required": ["file_path", "content"]
                    }),
                },
                Tool {
                    name: "delete_file".to_string(),
                    description: "Delete a file safely".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to delete"
                            }
                        },
                        "required": ["file_path"]
                    }),
                },
            ]);
        }

        if self.config.enable_directory_listing {
            tools.push(Tool {
                name: "list_directory".to_string(),
                description: "List contents of a directory".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "directory_path": {
                            "type": "string",
                            "description": "Path to the directory to list"
                        },
                        "include_hidden": {
                            "type": "boolean",
                            "description": "Whether to include hidden files",
                            "default": false
                        }
                    },
                    "required": ["directory_path"]
                }),
            });
        }

        tools
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        match name {
            "read_file" => self.read_file(arguments).await,
            "write_file" => self.write_file(arguments).await,
            "delete_file" => self.delete_file(arguments).await,
            "list_directory" => self.list_directory(arguments).await,
            "get_file_info" => self.get_file_info(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn read_file(&self, arguments: Value) -> Result<Value, String> {
        let request: ReadFileRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let path = self
            .validate_path(&request.file_path)
            .map_err(|e| e.to_string())?;

        let content = async_fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        self.validate_file_size(content.len() as u64)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "content": content,
            "path": path.to_string_lossy(),
            "size": content.len(),
            "encoding": "utf-8"
        }))
    }

    async fn write_file(&self, arguments: Value) -> Result<Value, String> {
        if self.config.read_only_mode {
            return Err("Server is in read-only mode".to_string());
        }

        let request: WriteFileRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        self.validate_file_size(request.content.len() as u64)
            .map_err(|e| e.to_string())?;

        let path = self
            .validate_path(&request.file_path)
            .map_err(|e| e.to_string())?;

        // Create parent directories if requested
        if request.create_directories.unwrap_or(false) {
            if let Some(parent) = path.parent() {
                async_fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
        }

        async_fs::write(&path, &request.content)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path.to_string_lossy(),
            "bytes_written": request.content.len(),
            "message": "File written successfully"
        }))
    }

    async fn delete_file(&self, arguments: Value) -> Result<Value, String> {
        if self.config.read_only_mode {
            return Err("Server is in read-only mode".to_string());
        }

        let request: DeleteFileRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let path = self
            .validate_path(&request.file_path)
            .map_err(|e| e.to_string())?;

        async_fs::remove_file(&path)
            .await
            .map_err(|e| format!("Failed to delete file: {}", e))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path.to_string_lossy(),
            "message": "File deleted successfully"
        }))
    }

    async fn list_directory(&self, arguments: Value) -> Result<Value, String> {
        if !self.config.enable_directory_listing {
            return Err("Directory listing is disabled".to_string());
        }

        let request: ListDirectoryRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let path = self
            .validate_path(&request.directory_path)
            .map_err(|e| e.to_string())?;

        let mut entries = async_fs::read_dir(&path)
            .await
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut files = Vec::new();
        let include_hidden = request.include_hidden.unwrap_or(false);

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let entry_path = entry.path();
            let name = entry_path.file_name().unwrap_or_default().to_string_lossy();

            // Skip hidden files unless requested
            if !include_hidden && name.starts_with('.') {
                continue;
            }

            match self.create_file_info(&entry_path).await {
                Ok(file_info) => files.push(file_info),
                Err(_) => continue, // Skip files we can't read
            }
        }

        let listing = DirectoryListing {
            path: path.to_string_lossy().to_string(),
            total_count: files.len(),
            files,
        };

        serde_json::to_value(listing).map_err(|e| format!("Failed to serialize listing: {}", e))
    }

    async fn get_file_info(&self, arguments: Value) -> Result<Value, String> {
        let request: ReadFileRequest = serde_json::from_value(arguments)
            .map_err(|e| format!("Failed to parse arguments: {}", e))?;

        let path = self
            .validate_path(&request.file_path)
            .map_err(|e| e.to_string())?;

        let file_info = self
            .create_file_info(&path)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_value(file_info).map_err(|e| format!("Failed to serialize file info: {}", e))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    eprintln!("📁 Starting File Operations MCP Server");
    eprintln!("=====================================");

    // Create config with safe defaults
    let config = FileOperationsConfig::default();

    eprintln!("⚙️  Security Configuration:");
    eprintln!("   Read-only mode: {}", config.read_only_mode);
    eprintln!("   Max file size: {} bytes", config.max_file_size);
    eprintln!("   Allowed extensions: {:?}", config.allowed_extensions);
    eprintln!("   Allowed directories: {:?}", config.allowed_directories);

    // Create server
    let server = FileOperationsServer::new(config);

    // Ensure demo directories exist
    for dir in ["./temp", "./data", "./examples"] {
        let _ = async_fs::create_dir_all(dir).await;
    }

    // Create demo files
    let demo_content = "This is a demo file created by the File Operations MCP Server.\nIt demonstrates safe file operations with security controls.";
    let _ = async_fs::write("./temp/demo.txt", demo_content).await;
    let _ = async_fs::write("./temp/config.json", r#"{"demo": true, "version": "1.0"}"#).await;

    // Demo file operations
    eprintln!("\n🧪 File Operations Demo:");

    // List tools
    let tools = server.list_tools();
    eprintln!("📋 Available tools ({}):", tools.len());
    for tool in &tools {
        eprintln!("  - {}: {}", tool.name, tool.description);
    }

    // Test read file
    eprintln!("\n📖 Reading demo file:");
    let read_args = serde_json::json!({
        "file_path": "./temp/demo.txt"
    });

    match server.call_tool("read_file", read_args).await {
        Ok(result) => {
            let content = result.get("content").unwrap_or(&Value::Null);
            eprintln!(
                "  ✅ Content: {}",
                content.as_str().unwrap_or("").lines().next().unwrap_or("")
            );
            eprintln!(
                "     Size: {} bytes",
                result.get("size").unwrap_or(&Value::Null)
            );
        }
        Err(e) => eprintln!("  ❌ Read failed: {}", e),
    }

    // Test list directory
    if server.config.enable_directory_listing {
        eprintln!("\n📂 Listing temp directory:");
        let list_args = serde_json::json!({
            "directory_path": "./temp"
        });

        match server.call_tool("list_directory", list_args).await {
            Ok(result) => {
                if let Ok(listing) = serde_json::from_value::<DirectoryListing>(result) {
                    eprintln!("  ✅ Found {} items:", listing.total_count);
                    for file in listing.files {
                        eprintln!(
                            "    - {}: {} ({} bytes)",
                            file.name, file.file_type, file.size
                        );
                    }
                }
            }
            Err(e) => eprintln!("  ❌ List failed: {}", e),
        }
    }

    // Test get file info
    eprintln!("\n📊 Getting file info:");
    let info_args = serde_json::json!({
        "file_path": "./temp/demo.txt"
    });

    match server.call_tool("get_file_info", info_args).await {
        Ok(result) => {
            if let Ok(info) = serde_json::from_value::<FileInfo>(result) {
                eprintln!("  ✅ File: {}", info.name);
                eprintln!("     Type: {}", info.file_type);
                eprintln!("     Size: {} bytes", info.size);
                eprintln!("     Modified: {}", info.modified);
            }
        }
        Err(e) => eprintln!("  ❌ Info failed: {}", e),
    }

    eprintln!("\n🎉 File operations demo completed!");
    eprintln!("\n🔒 Security features demonstrated:");
    eprintln!("   ✅ Path validation and sanitization");
    eprintln!("   ✅ Directory traversal prevention");
    eprintln!("   ✅ File extension filtering");
    eprintln!("   ✅ File size limits");
    eprintln!("   ✅ Read-only mode support");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_operations_server() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let config = FileOperationsConfig {
            allowed_directories: vec![temp_path.clone()],
            max_file_size: 1024,
            allowed_extensions: vec![".txt".to_string()],
            read_only_mode: false,
            enable_directory_listing: true,
        };

        let server = FileOperationsServer::new(config);

        // Test tools listing
        let tools = server.list_tools();
        assert!(tools.len() >= 3);
        assert!(tools.iter().any(|t| t.name == "read_file"));
        assert!(tools.iter().any(|t| t.name == "write_file"));
        assert!(tools.iter().any(|t| t.name == "list_directory"));
    }

    #[tokio::test]
    async fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileOperationsConfig {
            allowed_directories: vec![temp_dir.path().to_path_buf()],
            ..Default::default()
        };

        let server = FileOperationsServer::new(config);

        // Valid path should work
        let valid_path = temp_dir.path().join("test.txt");
        std::fs::write(&valid_path, "test").unwrap();

        let result = server.validate_path(&valid_path.to_string_lossy());
        assert!(result.is_ok());

        // Path outside allowed directory should fail
        let invalid_path = "/etc/passwd";
        let result = server.validate_path(invalid_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileOperationsConfig {
            allowed_directories: vec![temp_dir.path().to_path_buf()],
            read_only_mode: true,
            ..Default::default()
        };

        let server = FileOperationsServer::new(config);

        // Write should fail in read-only mode
        let write_args = serde_json::json!({
            "file_path": temp_dir.path().join("test.txt").to_string_lossy(),
            "content": "test content"
        });

        let result = server.call_tool("write_file", write_args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));
    }
}
