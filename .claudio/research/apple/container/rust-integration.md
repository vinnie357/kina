# Rust Integration Patterns for Apple Container

**Focus**: Swift-Rust interoperability approaches and FFI binding strategies

## Swift-Rust Interoperability Options

### 1. swift-bridge Crate (Recommended)

**Installation and Setup:**
```rust
// Cargo.toml
[dependencies]
swift-bridge = "0.1"

[build-dependencies]
swift-bridge-build = "0.1"
```

**Benefits:**
- Automatic FFI binding generation
- Memory-safe interop without serialization overhead
- Swift 6.0+ compatibility guaranteed
- Active maintenance and CNCF ecosystem compatibility

**Implementation Example:**
```rust
// kina-cli/src/core/apple_container.rs
use swift_bridge::*;

#[swift_bridge::bridge]
mod ffi {
    extern "Swift" {
        // Apple Containerization framework bindings
        type ContainerRuntime;

        #[swift_bridge(init)]
        fn new() -> ContainerRuntime;

        fn create_container(&self, image: &str, config: &str) -> Result<String, String>;
        fn start_container(&self, container_id: &str) -> Result<(), String>;
        fn stop_container(&self, container_id: &str) -> Result<(), String>;
        fn remove_container(&self, container_id: &str) -> Result<(), String>;

        fn pull_image(&self, image: &str) -> Result<(), String>;
        fn list_containers(&self) -> Result<Vec<String>, String>;
    }

    extern "Rust" {
        // Rust-side implementation
        type KinaRuntime;

        #[swift_bridge(init)]
        fn new() -> KinaRuntime;

        fn execute_kubernetes_operation(&self, operation: &str) -> Result<String, String>;
    }
}
```

### 2. UniFFI Integration

**Alternative Approach:**
```rust
// Alternative approach using UniFFI
use uniffi_bindgen::generate_bindings;

// Generate Swift bindings from Rust interface
uniffi_bindgen::generate_swift_bindings(&udl_path, &out_dir)?;
```

**Benefits:**
- Cross-platform binding generation (Swift, Kotlin, Python)
- Mozilla-maintained with strong ecosystem support
- Elegant C bridge abstraction

**UDL Interface Definition:**
```idl
// apple_container.udl
namespace apple_container {
    ContainerRuntime create_runtime();
};

interface ContainerRuntime {
    constructor();
    string create_container(string image, ContainerConfig config);
    void start_container(string container_id);
    void stop_container(string container_id);
    void remove_container(string container_id);
};

dictionary ContainerConfig {
    string image;
    sequence<string> command;
    record<string, string> environment;
    sequence<VolumeMount> volumes;
};
```

### 3. Direct FFI Implementation

**For Fine-Grained Control:**
```rust
// Direct C-style FFI for fine-grained control
use std::ffi::{CStr, CString, c_char};
use std::ptr;

#[no_mangle]
pub extern "C" fn kina_container_create(
    image_name: *const c_char,
    container_id: *mut *mut c_char,
) -> i32 {
    let image = unsafe {
        match CStr::from_ptr(image_name).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match create_container_internal(image) {
        Ok(id) => {
            let id_cstring = CString::new(id).unwrap();
            unsafe {
                *container_id = id_cstring.into_raw();
            }
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn kina_container_start(container_id: *const c_char) -> i32 {
    let id = unsafe {
        match CStr::from_ptr(container_id).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match start_container_internal(id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
```

## Recommended Integration Architecture

### Core FFI Module Structure

```rust
// kina-cli/src/core/apple_container.rs
use swift_bridge::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub volumes: Vec<VolumeMount>,
    pub privileged: bool,
    pub resource_limits: Option<ResourceLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
    pub disk_gb: Option<u64>,
}

#[swift_bridge::bridge]
mod ffi {
    extern "Swift" {
        type AppleContainerRuntime;

        #[swift_bridge(init)]
        fn new() -> AppleContainerRuntime;

        // Container lifecycle operations
        async fn create_container(
            &self,
            config_json: &str
        ) -> Result<String, String>;

        async fn start_container(
            &self,
            container_id: &str
        ) -> Result<(), String>;

        async fn stop_container(
            &self,
            container_id: &str
        ) -> Result<(), String>;

        async fn remove_container(
            &self,
            container_id: &str
        ) -> Result<(), String>;

        // Container inspection and logs
        async fn inspect_container(
            &self,
            container_id: &str
        ) -> Result<String, String>; // JSON response

        async fn container_logs(
            &self,
            container_id: &str,
            follow: bool
        ) -> Result<String, String>;

        // Image operations
        async fn pull_image(&self, image: &str) -> Result<(), String>;
        async fn list_images(&self) -> Result<Vec<String>, String>;
        async fn remove_image(&self, image: &str) -> Result<(), String>;

        // System operations
        async fn list_containers(&self) -> Result<String, String>; // JSON response
        async fn system_info(&self) -> Result<String, String>;
    }

    extern "Rust" {
        type KinaContainerManager;

        #[swift_bridge(init)]
        fn new() -> KinaContainerManager;

        async fn execute_kubernetes_command(
            &self,
            command: &str,
            args: Vec<String>
        ) -> Result<String, String>;

        async fn apply_kubernetes_manifest(
            &self,
            manifest_yaml: &str
        ) -> Result<String, String>;
    }
}

// Rust implementation
pub struct KinaContainerManager {
    swift_runtime: ffi::AppleContainerRuntime,
    container_cache: RwLock<HashMap<String, ContainerInfo>>,
}

impl KinaContainerManager {
    pub fn new() -> Self {
        Self {
            swift_runtime: ffi::AppleContainerRuntime::new(),
            container_cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_container(
        &self,
        config: &ContainerConfig
    ) -> Result<String, crate::errors::KinaError> {
        let config_json = serde_json::to_string(config)
            .map_err(|e| crate::errors::KinaError::SerializationError(e.to_string()))?;

        let container_id = self.swift_runtime
            .create_container(&config_json)
            .await
            .map_err(|e| crate::errors::KinaError::ContainerError(e))?;

        // Update cache
        let mut cache = self.container_cache.write().await;
        cache.insert(container_id.clone(), ContainerInfo {
            id: container_id.clone(),
            config: config.clone(),
            state: ContainerState::Created,
        });

        Ok(container_id)
    }

    pub async fn start_container(
        &self,
        container_id: &str
    ) -> Result<(), crate::errors::KinaError> {
        self.swift_runtime
            .start_container(container_id)
            .await
            .map_err(|e| crate::errors::KinaError::ContainerError(e))?;

        // Update cache
        let mut cache = self.container_cache.write().await;
        if let Some(info) = cache.get_mut(container_id) {
            info.state = ContainerState::Running;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ContainerInfo {
    id: String,
    config: ContainerConfig,
    state: ContainerState,
}

#[derive(Debug, Clone)]
enum ContainerState {
    Created,
    Running,
    Stopped,
    Error(String),
}
```

## Build System Integration

### Cargo Configuration

```toml
# kina-cli/Cargo.toml
[build-dependencies]
swift-bridge-build = "0.1"

[dependencies]
swift-bridge = "0.1"
tokio = { version = "1.0", features = ["full"] }
kube = { version = "0.87", features = ["runtime", "derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Build Script Implementation

```rust
// build.rs
use swift_bridge_build::*;
use std::path::PathBuf;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Generate Swift-Rust bindings
    let bridges = swift_bridge_build::parse_bridges(vec![
        "src/core/apple_container.rs"
    ])
    .unwrap();

    bridges
        .generate()
        .write_all_concatenated(&out_dir, env!("CARGO_PKG_NAME"));

    // Link Apple frameworks
    println!("cargo:rustc-link-framework=Virtualization");
    println!("cargo:rustc-link-framework=Foundation");

    // Set up Swift package integration
    let swift_package_path = PathBuf::from("swift-package");
    if swift_package_path.exists() {
        println!("cargo:rustc-link-search=native={}/build", swift_package_path.display());
        println!("cargo:rustc-link-lib=static=KinaSwiftBridge");
    }
}
```

### Swift Package Configuration

```swift
// swift-package/Package.swift
// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "KinaSwiftBridge",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .library(
            name: "KinaSwiftBridge",
            type: .static,
            targets: ["KinaSwiftBridge"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/container.git", branch: "main")
    ],
    targets: [
        .target(
            name: "KinaSwiftBridge",
            dependencies: [
                .product(name: "Containerization", package: "container")
            ]
        )
    ]
)
```

## Memory Management and Safety

### Rust-Swift Memory Safety Patterns

```rust
// Safe memory management across FFI boundary
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SafeContainerHandle {
    inner: Arc<Mutex<ffi::AppleContainerRuntime>>,
}

impl SafeContainerHandle {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ffi::AppleContainerRuntime::new())),
        }
    }

    pub async fn create_container_safe(
        &self,
        config: ContainerConfig
    ) -> Result<String, crate::errors::KinaError> {
        let runtime = self.inner.lock().await;

        // Serialize config to JSON for safe FFI transfer
        let config_json = serde_json::to_string(&config)
            .map_err(|e| crate::errors::KinaError::SerializationError(e.to_string()))?;

        // Call Swift function with JSON string (safe across FFI)
        let result = runtime.create_container(&config_json).await
            .map_err(|e| crate::errors::KinaError::ContainerError(e))?;

        Ok(result)
    }
}

// Clone implementation for thread safety
impl Clone for SafeContainerHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
```

### Error Handling Across FFI

```rust
// Robust error handling for Swift-Rust integration
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppleContainerError {
    #[error("Container creation failed: {0}")]
    CreationFailed(String),

    #[error("Container not found: {id}")]
    ContainerNotFound { id: String },

    #[error("Swift runtime error: {0}")]
    SwiftRuntimeError(String),

    #[error("JSON serialization error: {0}")]
    SerializationError(String),

    #[error("FFI communication error: {0}")]
    FFIError(String),
}

impl From<String> for AppleContainerError {
    fn from(error: String) -> Self {
        if error.contains("not found") {
            // Parse container ID from error message
            AppleContainerError::ContainerNotFound {
                id: extract_container_id(&error).unwrap_or("unknown".to_string())
            }
        } else if error.contains("creation") {
            AppleContainerError::CreationFailed(error)
        } else {
            AppleContainerError::SwiftRuntimeError(error)
        }
    }
}

fn extract_container_id(error_msg: &str) -> Option<String> {
    // Parse container ID from Swift error message
    error_msg
        .split_whitespace()
        .find(|&word| word.len() == 64 && word.chars().all(|c| c.is_ascii_hexdigit()))
        .map(String::from)
}
```

## Async Integration Patterns

### Tokio-Swift Async Interop

```rust
// Async bridge between Rust and Swift
use tokio::task;
use futures::Future;

pub struct AsyncContainerManager {
    handle: SafeContainerHandle,
}

impl AsyncContainerManager {
    pub fn new() -> Self {
        Self {
            handle: SafeContainerHandle::new(),
        }
    }

    pub async fn create_and_start_container(
        &self,
        config: ContainerConfig
    ) -> Result<String, AppleContainerError> {
        // Create container
        let container_id = self.handle.create_container_safe(config).await?;

        // Start container
        self.handle.start_container(&container_id).await?;

        Ok(container_id)
    }

    pub async fn run_container_with_timeout(
        &self,
        config: ContainerConfig,
        timeout: std::time::Duration
    ) -> Result<String, AppleContainerError> {
        // Run with timeout
        let result = tokio::time::timeout(
            timeout,
            self.create_and_start_container(config)
        ).await;

        match result {
            Ok(container_result) => container_result,
            Err(_) => Err(AppleContainerError::FFIError(
                "Container creation timed out".to_string()
            )),
        }
    }
}
```

## Testing FFI Integration

### Unit Tests for FFI Bindings

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_container_lifecycle() {
        let manager = AsyncContainerManager::new();

        let config = ContainerConfig {
            image: "alpine:latest".to_string(),
            command: vec!["sleep".to_string(), "30".to_string()],
            environment: HashMap::new(),
            volumes: vec![],
            privileged: false,
            resource_limits: None,
        };

        // Test container creation and startup
        let container_id = manager.create_and_start_container(config).await.unwrap();
        assert!(!container_id.is_empty());

        // Test container cleanup
        manager.handle.stop_container(&container_id).await.unwrap();
        manager.handle.remove_container(&container_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_error_handling() {
        let manager = AsyncContainerManager::new();

        let invalid_config = ContainerConfig {
            image: "nonexistent:latest".to_string(),
            command: vec![],
            environment: HashMap::new(),
            volumes: vec![],
            privileged: false,
            resource_limits: None,
        };

        // Should fail gracefully
        let result = manager.create_and_start_container(invalid_config).await;
        assert!(result.is_err());
    }
}