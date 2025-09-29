# Development Workflows and Testing

**Focus**: Testing strategies, CI/CD integration, and development environment setup

## Development Environment Setup

### Prerequisites and System Requirements

```bash
# System requirements verification
sw_vers -productVersion  # Verify macOS 26+
uname -m                 # Verify Apple Silicon (arm64)
xcode-select -p          # Verify Xcode installation
swift --version          # Verify Swift compiler
```

### Development Environment Configuration

```bash
# Rust toolchain with Apple Silicon support
rustup target add aarch64-apple-darwin
rustup component add clippy rustfmt

# Install required development tools
brew install protobuf grpcurl jq
cargo install cargo-watch cargo-audit

# Apple Container CLI installation
git clone https://github.com/apple/container.git
cd container
swift build -c release
ln -sf $(pwd)/.build/release/container /usr/local/bin/container

# Verify installation
container --version
```

### Project Development Setup

```bash
# Clone and setup kina project
git clone <kina-repo>
cd kina

# Setup development environment with mise
mise install

# Build development version
cargo build --target aarch64-apple-darwin

# Setup Git hooks for development
cp scripts/git-hooks/* .git/hooks/
chmod +x .git/hooks/*
```

### Cargo Configuration for Apple Container Integration

```toml
# .cargo/config.toml
[build]
target = "aarch64-apple-darwin"

[target.aarch64-apple-darwin]
rustflags = [
    "-C", "link-arg=-framework",
    "-C", "link-arg=Virtualization",
    "-C", "link-arg=-framework",
    "-C", "link-arg=Foundation",
]

[env]
# Swift package integration
SWIFT_PACKAGE_PATH = { value = "./swift-package", relative = true }

# Apple Container development flags
CONTAINER_DEBUG = "1"
KINA_LOG_LEVEL = "debug"
```

## Testing Strategy and Framework

### Unit Testing Architecture

```rust
// kina-cli/tests/unit/apple_container_tests.rs
use kina_cli::core::apple_container::*;
use tokio_test;
use std::collections::HashMap;

#[tokio::test]
async fn test_container_lifecycle() {
    let manager = KinaContainerManager::new();

    let config = ContainerConfig {
        image: "alpine:latest".to_string(),
        command: vec!["sleep".to_string(), "30".to_string()],
        environment: HashMap::new(),
        volumes: vec![],
        privileged: false,
        resource_limits: Some(ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 512,
            disk_gb: 5,
        }),
    };

    // Test container creation
    let container_id = manager.create_container(&config).await.unwrap();
    assert!(!container_id.is_empty());
    assert_eq!(container_id.len(), 64); // Expected container ID length

    // Test container start
    manager.start_container(&container_id).await.unwrap();

    // Verify container is running
    let status = manager.get_container_status(&container_id).await.unwrap();
    assert_eq!(status.state, ContainerState::Running);

    // Test container execution
    let result = manager.exec_container(
        &container_id,
        &["echo", "hello", "world"]
    ).await.unwrap();
    assert_eq!(result.trim(), "hello world");

    // Test container stop
    manager.stop_container(&container_id).await.unwrap();

    // Test container cleanup
    manager.remove_container(&container_id).await.unwrap();
}

#[tokio::test]
async fn test_error_handling() {
    let manager = KinaContainerManager::new();

    // Test invalid image
    let invalid_config = ContainerConfig {
        image: "nonexistent:latest".to_string(),
        command: vec![],
        environment: HashMap::new(),
        volumes: vec![],
        privileged: false,
        resource_limits: None,
    };

    let result = manager.create_container(&invalid_config).await;
    assert!(result.is_err());

    // Test invalid container ID operations
    let result = manager.start_container("invalid-id").await;
    assert!(result.is_err());

    let result = manager.stop_container("invalid-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_resource_limits() {
    let manager = KinaContainerManager::new();

    let config = ContainerConfig {
        image: "alpine:latest".to_string(),
        command: vec!["sh", "-c", "stress --vm 1 --vm-bytes 100M --timeout 10s"],
        environment: HashMap::new(),
        volumes: vec![],
        privileged: false,
        resource_limits: Some(ResourceLimits {
            cpu_cores: 0.5,
            memory_mb: 128, // Limited memory
            disk_gb: 1,
        }),
    };

    let container_id = manager.create_container(&config).await.unwrap();
    manager.start_container(&container_id).await.unwrap();

    // Wait for container to complete
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Verify container exited successfully (didn't OOM)
    let status = manager.get_container_status(&container_id).await.unwrap();
    assert_eq!(status.state, ContainerState::Exited);
    assert_eq!(status.exit_code, Some(0));

    manager.remove_container(&container_id).await.unwrap();
}
```

### Integration Testing Framework

```rust
// kina-cli/tests/integration/cluster_tests.rs
use kina_cli::core::cluster::*;
use kina_cli::core::kubernetes::*;
use std::time::Duration;

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_single_node_cluster_creation() {
    let cluster_config = ClusterConfig {
        name: "test-cluster".to_string(),
        worker_nodes: 0,
        kubernetes_version: "v1.28.0".to_string(),
        node_image: "kindest/node:v1.28.0".to_string(),
        control_plane_endpoint: None,
        pod_subnet: "10.244.0.0/16".to_string(),
        service_subnet: "10.96.0.0/12".to_string(),
        cni_plugin: "cilium".to_string(),
        feature_gates: vec![],
        resource_limits: ResourceLimits {
            cpu_cores: 2.0,
            memory_mb: 2048,
            disk_gb: 20,
        },
    };

    let mut cluster = KinaCluster::new(cluster_config).await.unwrap();

    // Create cluster
    cluster.create().await.unwrap();

    // Wait for cluster to be ready
    cluster.wait_for_ready().await.unwrap();

    // Verify cluster is accessible
    let k8s_client = cluster.kubernetes_client().await.unwrap();
    let nodes = k8s_client.list_nodes().await.unwrap();
    assert_eq!(nodes.len(), 1);

    // Verify node is ready
    let node = &nodes[0];
    assert_eq!(node.status.phase, Some("Ready".to_string()));

    // Test basic pod creation
    let pod_manifest = r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
  - name: test-container
    image: alpine:latest
    command: ["sleep", "30"]
"#;

    k8s_client.apply_manifest(pod_manifest).await.unwrap();

    // Wait for pod to be running
    let mut attempts = 0;
    loop {
        let pods = k8s_client.list_pods("default").await.unwrap();
        let test_pod = pods.iter().find(|p| p.name == "test-pod");

        if let Some(pod) = test_pod {
            if pod.status.phase == Some("Running".to_string()) {
                break;
            }
        }

        attempts += 1;
        if attempts > 30 {
            panic!("Pod failed to start within timeout");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Cleanup
    k8s_client.delete_pod("default", "test-pod").await.unwrap();
    cluster.delete().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_multi_node_cluster() {
    let cluster_config = ClusterConfig {
        name: "multi-node-test".to_string(),
        worker_nodes: 2,
        kubernetes_version: "v1.28.0".to_string(),
        node_image: "kindest/node:v1.28.0".to_string(),
        control_plane_endpoint: None,
        pod_subnet: "10.244.0.0/16".to_string(),
        service_subnet: "10.96.0.0/12".to_string(),
        cni_plugin: "cilium".to_string(),
        feature_gates: vec![],
        resource_limits: ResourceLimits {
            cpu_cores: 1.0,
            memory_mb: 1024,
            disk_gb: 10,
        },
    };

    let mut cluster = KinaCluster::new(cluster_config).await.unwrap();
    cluster.create().await.unwrap();
    cluster.wait_for_ready().await.unwrap();

    let k8s_client = cluster.kubernetes_client().await.unwrap();
    let nodes = k8s_client.list_nodes().await.unwrap();

    // Verify we have 3 nodes (1 control plane + 2 workers)
    assert_eq!(nodes.len(), 3);

    // Verify all nodes are ready
    for node in &nodes {
        assert_eq!(node.status.phase, Some("Ready".to_string()));
    }

    // Test pod scheduling across nodes
    let deployment_manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-deployment
spec:
  replicas: 3
  selector:
    matchLabels:
      app: test-app
  template:
    metadata:
      labels:
        app: test-app
    spec:
      containers:
      - name: test-container
        image: nginx:alpine
        ports:
        - containerPort: 80
"#;

    k8s_client.apply_manifest(deployment_manifest).await.unwrap();

    // Wait for all pods to be scheduled
    let mut attempts = 0;
    loop {
        let pods = k8s_client.list_pods("default").await.unwrap();
        let test_pods: Vec<_> = pods.iter()
            .filter(|p| p.labels.get("app") == Some(&"test-app".to_string()))
            .collect();

        if test_pods.len() == 3 && test_pods.iter().all(|p| p.status.phase == Some("Running".to_string())) {
            // Verify pods are scheduled on different nodes
            let node_names: std::collections::HashSet<_> = test_pods.iter()
                .map(|p| p.spec.node_name.as_ref().unwrap())
                .collect();
            assert!(node_names.len() >= 2, "Pods should be distributed across nodes");
            break;
        }

        attempts += 1;
        if attempts > 60 {
            panic!("Deployment failed to start within timeout");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    // Cleanup
    k8s_client.delete_deployment("default", "test-deployment").await.unwrap();
    cluster.delete().await.unwrap();
}
```

### CRI Compliance Testing

```rust
// kina-cli/tests/integration/cri_tests.rs
use kina_cli::core::cri_shim::*;
use k8s_cri::v1::*;
use tonic::Request;

#[tokio::test]
#[ignore]
async fn test_cri_compliance() {
    let shim = AppleContainerCRIShim::new(
        kina_cli::core::apple_container::KinaContainerManager::new()
    );

    // Test RuntimeService
    test_runtime_service(&shim).await;

    // Test ImageService
    test_image_service(&shim).await;
}

async fn test_runtime_service(shim: &AppleContainerCRIShim) {
    // Test Version
    let version_response = shim.version(Request::new(VersionRequest {})).await.unwrap();
    let version = version_response.into_inner();
    assert!(version.version.contains("kina"));
    assert!(version.runtime_name.contains("apple-container"));

    // Test PodSandbox lifecycle
    let pod_config = PodSandboxConfig {
        metadata: Some(PodSandboxMetadata {
            name: "test-pod".to_string(),
            namespace: "default".to_string(),
            uid: "test-uid".to_string(),
            ..Default::default()
        }),
        hostname: "test-hostname".to_string(),
        log_directory: "/var/log/pods".to_string(),
        dns_config: Some(DnsConfig {
            servers: vec!["8.8.8.8".to_string()],
            searches: vec!["default.svc.cluster.local".to_string()],
            options: vec![],
        }),
        ..Default::default()
    };

    let run_request = RunPodSandboxRequest {
        config: Some(pod_config.clone()),
        runtime_handler: String::new(),
    };

    let response = shim.run_pod_sandbox(Request::new(run_request)).await.unwrap();
    let pod_sandbox_id = response.into_inner().pod_sandbox_id;
    assert!(!pod_sandbox_id.is_empty());

    // Test PodSandbox status
    let status_request = PodSandboxStatusRequest {
        pod_sandbox_id: pod_sandbox_id.clone(),
        verbose: false,
    };

    let status_response = shim.pod_sandbox_status(Request::new(status_request)).await.unwrap();
    let status = status_response.into_inner().status.unwrap();
    assert_eq!(status.state, pod_sandbox_state::State::PodSandboxReady as i32);

    // Test Container creation
    let container_config = ContainerConfig {
        metadata: Some(ContainerMetadata {
            name: "test-container".to_string(),
            ..Default::default()
        }),
        image: Some(ImageSpec {
            image: "alpine:latest".to_string(),
            ..Default::default()
        }),
        command: vec!["sleep".to_string(), "300".to_string()],
        args: vec![],
        working_dir: String::new(),
        envs: vec![],
        mounts: vec![],
        devices: vec![],
        labels: std::collections::HashMap::new(),
        annotations: std::collections::HashMap::new(),
        log_path: String::new(),
        stdin: false,
        stdin_once: false,
        tty: false,
        linux: None,
        windows: None,
    };

    let create_request = CreateContainerRequest {
        pod_sandbox_id: pod_sandbox_id.clone(),
        config: Some(container_config),
        sandbox_config: Some(pod_config),
    };

    let create_response = shim.create_container(Request::new(create_request)).await.unwrap();
    let container_id = create_response.into_inner().container_id;
    assert!(!container_id.is_empty());

    // Test Container start
    let start_request = StartContainerRequest {
        container_id: container_id.clone(),
    };

    shim.start_container(Request::new(start_request)).await.unwrap();

    // Test Container status
    let container_status_request = ContainerStatusRequest {
        container_id: container_id.clone(),
        verbose: false,
    };

    let container_status_response = shim.container_status(Request::new(container_status_request)).await.unwrap();
    let container_status = container_status_response.into_inner().status.unwrap();
    assert_eq!(container_status.state, container_state::State::ContainerRunning as i32);

    // Cleanup
    let stop_request = StopContainerRequest {
        container_id: container_id.clone(),
        timeout: 10,
    };
    shim.stop_container(Request::new(stop_request)).await.unwrap();

    let remove_request = RemoveContainerRequest {
        container_id,
    };
    shim.remove_container(Request::new(remove_request)).await.unwrap();

    let stop_sandbox_request = StopPodSandboxRequest {
        pod_sandbox_id: pod_sandbox_id.clone(),
    };
    shim.stop_pod_sandbox(Request::new(stop_sandbox_request)).await.unwrap();

    let remove_sandbox_request = RemovePodSandboxRequest {
        pod_sandbox_id,
    };
    shim.remove_pod_sandbox(Request::new(remove_sandbox_request)).await.unwrap();
}

async fn test_image_service(shim: &AppleContainerCRIShim) {
    // Test image pull
    let pull_request = PullImageRequest {
        image: Some(ImageSpec {
            image: "alpine:latest".to_string(),
            ..Default::default()
        }),
        auth: None,
        sandbox_config: None,
    };

    shim.pull_image(Request::new(pull_request)).await.unwrap();

    // Test list images
    let list_request = ListImagesRequest {
        filter: None,
    };

    let list_response = shim.list_images(Request::new(list_request)).await.unwrap();
    let images = list_response.into_inner().images;
    assert!(!images.is_empty());

    // Verify alpine image is in the list
    let alpine_image = images.iter().find(|img| {
        img.repo_tags.iter().any(|tag| tag.contains("alpine"))
    });
    assert!(alpine_image.is_some());

    // Test image status
    let status_request = ImageStatusRequest {
        image: Some(ImageSpec {
            image: "alpine:latest".to_string(),
            ..Default::default()
        }),
        verbose: false,
    };

    let status_response = shim.image_status(Request::new(status_request)).await.unwrap();
    let image_status = status_response.into_inner().image;
    assert!(image_status.is_some());
}
```

## Continuous Integration and Deployment

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: kina CI/CD

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    runs-on: macos-14-xlarge  # Apple Silicon runner
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin
          components: clippy, rustfmt

      - name: Cache Cargo dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install system dependencies
        run: |
          brew install protobuf
          # Note: Apple Container requires macOS 26+ beta
          # This will need to be updated when available in CI

      - name: Check code formatting
        run: cargo fmt --all -- --check

      - name: Run Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run unit tests
        run: cargo test --lib --target aarch64-apple-darwin

      - name: Run integration tests
        run: cargo test --test '*' --target aarch64-apple-darwin -- --ignored
        if: false  # Disabled until macOS 26+ available in CI

      - name: Run security audit
        run: cargo audit

  build:
    needs: test
    runs-on: macos-14-xlarge
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin

      - name: Build release binary
        run: cargo build --release --target aarch64-apple-darwin

      - name: Run smoke tests
        run: |
          ./target/aarch64-apple-darwin/release/kina --version
          ./target/aarch64-apple-darwin/release/kina --help

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: kina-binary
          path: target/aarch64-apple-darwin/release/kina

  release:
    if: github.ref == 'refs/heads/main'
    needs: build
    runs-on: macos-14-xlarge
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v3
        with:
          name: kina-binary

      - name: Create release
        uses: softprops/action-gh-release@v1
        if: startsWith(github.ref, 'refs/tags/')
        with:
          files: kina
          generate_release_notes: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Development Scripts

```bash
#!/bin/bash
# scripts/dev-setup.sh - Development environment setup

set -euo pipefail

echo "Setting up kina development environment..."

# Check system requirements
if [[ $(uname -m) != "arm64" ]]; then
    echo "Error: Apple Silicon (ARM64) required"
    exit 1
fi

if [[ $(sw_vers -productVersion | cut -d. -f1) -lt 26 ]]; then
    echo "Warning: macOS 26+ recommended for Apple Container support"
fi

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# Add Apple Silicon target
rustup target add aarch64-apple-darwin
rustup component add clippy rustfmt

# Install development tools
cargo install cargo-watch cargo-audit cargo-nextest

# Setup Git hooks
if [[ -d .git ]]; then
    cp scripts/git-hooks/* .git/hooks/
    chmod +x .git/hooks/*
    echo "Git hooks installed"
fi

# Build project
echo "Building kina..."
cargo build --target aarch64-apple-darwin

echo "Development environment setup complete!"
echo "Run 'cargo test' to run unit tests"
echo "Run 'cargo test -- --ignored' to run integration tests (requires macOS 26+)"
```

```bash
#!/bin/bash
# scripts/test-runner.sh - Comprehensive test runner

set -euo pipefail

echo "Running kina test suite..."

# Unit tests
echo "Running unit tests..."
cargo test --lib --target aarch64-apple-darwin

# Integration tests (requires macOS 26+)
if [[ $(sw_vers -productVersion | cut -d. -f1) -ge 26 ]]; then
    echo "Running integration tests..."
    cargo test --test '*' --target aarch64-apple-darwin -- --ignored
else
    echo "Skipping integration tests (requires macOS 26+)"
fi

# Performance tests
echo "Running performance benchmarks..."
cargo bench --target aarch64-apple-darwin

# Security audit
echo "Running security audit..."
cargo audit

# Code quality checks
echo "Running code quality checks..."
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

echo "All tests completed successfully!"
```

### Local Development Workflow

```bash
# Daily development workflow commands

# Start development with file watching
cargo watch -x 'build --target aarch64-apple-darwin'

# Run tests continuously
cargo watch -x 'test --lib'

# Run specific test
cargo test test_container_lifecycle -- --exact

# Run integration tests
cargo test --test cluster_tests -- --ignored

# Performance profiling
cargo build --release --target aarch64-apple-darwin
sudo dtrace -s scripts/profile.d -p $(pgrep kina)

# Memory debugging
cargo build --target aarch64-apple-darwin
valgrind --tool=memcheck target/aarch64-apple-darwin/debug/kina

# Network debugging
sudo tcpdump -i any -w kina-network.pcap
```

This comprehensive development and testing framework provides the foundation for reliable Apple Container integration development with proper CI/CD pipeline support.