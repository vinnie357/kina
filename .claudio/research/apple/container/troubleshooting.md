# Apple Container Troubleshooting Guide

**Focus**: Common issues, solutions, and debugging strategies for Apple Container integration

## System Requirements Issues

### macOS Version Compatibility

**Problem**: "Apple Container framework not available"
```bash
Error: dyld[1234]: Symbol not found: _OBJC_CLASS_$_VZVirtualMachine
```

**Solution**:
```bash
# Check macOS version
sw_vers -productVersion

# Required: macOS 26.0+ beta
# Install beta from Apple Developer portal
```

**Verification**:
```bash
# Verify Virtualization framework availability
system_profiler SPSoftwareDataType | grep "System Version"
ls /System/Library/Frameworks/Virtualization.framework
```

### Apple Silicon Requirements

**Problem**: "Virtualization not supported on this platform"
```bash
Error: Apple Container requires Apple Silicon (ARM64) architecture
```

**Solution**:
```bash
# Check processor architecture
uname -m
# Should output: arm64

# Check virtualization support
sysctl -n machdep.cpu.features | grep -i virtual
```

### Entitlements and Permissions

**Problem**: "Permission denied accessing Virtualization framework"
```bash
Error: com.apple.security.virtualization entitlement required
```

**Solution**:
```xml
<!-- Add to app entitlements -->
<key>com.apple.security.virtualization</key>
<true/>
<key>com.apple.security.network.client</key>
<true/>
<key>com.apple.security.files.user-selected.read-write</key>
<true/>
```

**Code signing**:
```bash
# Sign with entitlements
codesign --force --sign "Developer ID" --entitlements kina.entitlements target/release/kina

# Verify entitlements
codesign -d --entitlements - target/release/kina
```

## Container Runtime Issues

### Container Creation Failures

**Problem**: "Failed to create container VM"
```bash
Error: VZVirtualMachine creation failed: insufficient resources
```

**Diagnosis**:
```bash
# Check available resources
vm_stat | head -5
top -l 1 | grep PhysMem

# Check running VMs
container list
ps aux | grep -i virtualization
```

**Solutions**:
```bash
# Free up memory
sudo purge

# Stop unnecessary containers
container stop $(container list -q)

# Adjust VM resource limits in config
export KINA_VM_MEMORY_MB=512  # Reduce from default
export KINA_VM_CPU_CORES=1    # Reduce from default
```

### Container Startup Timeouts

**Problem**: "Container failed to start within timeout"
```bash
Error: Container startup timeout after 60 seconds
```

**Debugging**:
```bash
# Enable debug logging
export CONTAINER_DEBUG=1
container --debug run alpine:latest

# Check container logs
container logs <container-id>

# Monitor VM startup
sudo log stream --predicate 'subsystem == "com.apple.virtualization"'
```

**Solutions**:
```rust
// Increase timeout in Rust code
let timeout = Duration::from_secs(120); // Increase from 60
let result = tokio::time::timeout(timeout, container_operation).await;
```

### Image Pull Failures

**Problem**: "Failed to pull container image"
```bash
Error: image pull failed: registry authentication required
```

**Solutions**:
```bash
# Login to registry
container registry login docker.io

# Check registry configuration
container registry list

# Use authenticated image pull
container pull docker.io/library/alpine:latest
```

**Registry configuration**:
```bash
# Configure registry mirrors
mkdir -p ~/.kina/config
cat > ~/.kina/config/registries.toml << EOF
[registry."docker.io"]
  mirrors = ["https://mirror.gcr.io"]
EOF
```

## Swift-Rust FFI Issues

### Build Failures

**Problem**: "Swift bridge compilation failed"
```bash
error: failed to build swift-bridge bindings
note: Swift compiler not found
```

**Solutions**:
```bash
# Install Xcode command line tools
xcode-select --install

# Verify Swift compiler
swift --version
which swift

# Check Xcode path
xcode-select -p
sudo xcode-select -s /Applications/Xcode-beta.app/Contents/Developer
```

### Memory Management Issues

**Problem**: "Segmentation fault in FFI boundary"
```bash
thread 'main' panicked at 'assertion failed'
note: run with `RUST_BACKTRACE=1` for a backtrace
```

**Debugging**:
```bash
# Enable Rust backtrace
export RUST_BACKTRACE=full
export RUST_LOG=debug

# Run with memory debugging
cargo test --features debug-ffi
```

**Solutions**:
```rust
// Fix common FFI memory issues
use std::ffi::CString;

// Ensure proper string lifetime management
let c_string = CString::new(rust_string).unwrap();
let result = unsafe { swift_function(c_string.as_ptr()) };
// c_string lives until here

// Use Arc for shared ownership across FFI
use std::sync::Arc;
let shared_data = Arc::new(data);
let shared_clone = Arc::clone(&shared_data);
```

### Swift Package Integration

**Problem**: "Swift package dependencies not found"
```bash
error: package 'container' is not found
```

**Solutions**:
```bash
# Clone Apple Container repository
git clone https://github.com/apple/container.git
cd container

# Build Swift package
swift build -c release

# Verify package location
ls .build/release/
```

**Package.swift configuration**:
```swift
// Ensure correct dependency specification
.package(url: "https://github.com/apple/container.git", branch: "main")
```

## Kubernetes Integration Issues

### CRI Shim Failures

**Problem**: "CRI server failed to start"
```bash
Error: failed to bind to unix socket /var/run/kina-cri.sock
```

**Solutions**:
```bash
# Check socket permissions
ls -la /var/run/kina-cri.sock
sudo rm /var/run/kina-cri.sock  # Remove stale socket

# Check socket directory permissions
sudo chmod 755 /var/run
sudo chown root:wheel /var/run

# Start CRI server with proper permissions
sudo kina cri-server --socket /var/run/kina-cri.sock
```

### kubelet Integration Problems

**Problem**: "kubelet cannot connect to runtime"
```bash
kubelet[1234]: failed to get runtime status: rpc error: code = Unavailable
```

**Debugging**:
```bash
# Test CRI server directly
grpcurl -plaintext -unix /var/run/kina-cri.sock \
  runtime.v1.RuntimeService/Status

# Check kubelet configuration
sudo cat /var/lib/kubelet/config.yaml | grep -A5 containerRuntime

# Verify socket connectivity
sudo lsof | grep kina-cri.sock
```

**Solutions**:
```yaml
# Fix kubelet configuration
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
containerRuntime: remote
containerRuntimeEndpoint: unix:///var/run/kina-cri.sock
imageServiceEndpoint: unix:///var/run/kina-cri.sock
```

### Pod Scheduling Issues

**Problem**: "Pods stuck in pending state"
```bash
kubectl get pods
NAME      READY   STATUS    RESTARTS   AGE
test-pod   0/1     Pending   0          5m
```

**Debugging**:
```bash
# Check pod events
kubectl describe pod test-pod

# Check node status
kubectl get nodes -o wide

# Check CRI logs
sudo journalctl -u kina-cri-server -f
```

**Common solutions**:
```bash
# Check resource availability
kubectl describe nodes | grep -A5 "Allocated resources"

# Verify CRI runtime status
kubectl get nodes -o jsonpath='{.items[*].status.nodeInfo.containerRuntimeVersion}'

# Check for pod specification issues
kubectl get pod test-pod -o yaml | grep -A10 spec
```

## Network Connectivity Issues

### Container Network Isolation

**Problem**: "Containers cannot reach external network"
```bash
# Inside container
ping google.com
ping: cannot resolve google.com: Name or service not known
```

**Debugging**:
```bash
# Check VM network configuration
container exec <container-id> ip addr show
container exec <container-id> cat /etc/resolv.conf

# Check host network
networksetup -getdnsservers "Wi-Fi"
route -n get default
```

**Solutions**:
```bash
# Fix DNS configuration
container exec <container-id> sh -c 'echo "nameserver 8.8.8.8" > /etc/resolv.conf'

# Verify network routes
container exec <container-id> ip route show

# Check firewall settings
sudo pfctl -sr | grep kina
```

### Pod-to-Pod Networking

**Problem**: "Pods cannot communicate with each other"
```bash
# From pod A
curl pod-b-service:8080
curl: (6) Could not resolve host: pod-b-service
```

**Debugging**:
```bash
# Check service discovery
kubectl get services
kubectl get endpoints

# Test DNS resolution
kubectl exec pod-a -- nslookup pod-b-service

# Check network policies
kubectl get networkpolicies
```

**Solutions**:
```yaml
# Verify service configuration
apiVersion: v1
kind: Service
metadata:
  name: pod-b-service
spec:
  selector:
    app: pod-b
  ports:
    - port: 8080
      targetPort: 8080
```

## Performance Issues

### Container Startup Performance

**Problem**: "Containers take too long to start"
```bash
# Measurement
time container run alpine:latest echo "hello"
# Output: real 0m15.231s (should be <1s)
```

**Profiling**:
```bash
# Enable performance tracing
export CONTAINER_TRACE=1
container --debug run alpine:latest

# Monitor VM creation time
sudo log stream --predicate 'subsystem == "com.apple.virtualization"' --level debug
```

**Optimization strategies**:
```bash
# Pre-pull images
container pull alpine:latest
container pull kindest/node:v1.28.0

# Use smaller base images
container run alpine:latest  # instead of ubuntu:latest

# Enable image caching
export KINA_IMAGE_CACHE_SIZE=10
```

### Memory Usage Issues

**Problem**: "High memory consumption per container"
```bash
# Each container VM uses >500MB memory
ps aux | grep -i virtualization
```

**Monitoring**:
```bash
# Monitor VM memory usage
sudo memory_pressure
vm_stat | grep "Pages active"

# Check container resource limits
container inspect <container-id> | grep -i memory
```

**Optimization**:
```rust
// Reduce VM memory allocation
pub struct ResourceLimits {
    pub memory_mb: u64, // Set to 256MB instead of 512MB
    pub cpu_cores: f64, // Set to 0.5 instead of 1.0
}
```

## Debugging Tools and Techniques

### Logging Configuration

```bash
# Enable comprehensive logging
export RUST_LOG=kina=debug,apple_container=trace
export CONTAINER_DEBUG=1
export KINA_LOG_LEVEL=debug

# Log to file
kina cluster create test-cluster 2>&1 | tee kina-debug.log
```

### System Monitoring

```bash
# Monitor system resources
sudo fs_usage -w | grep kina
sudo dtruss -p $(pgrep kina)

# Monitor network activity
sudo tcpdump -i any | grep kina

# Monitor file system activity
sudo opensnoop -n kina
```

### Container Inspection

```bash
# Comprehensive container inspection
container inspect <container-id> | jq '.'

# VM process inspection
ps aux | grep -E "(kina|container)"
pstree -p | grep -A5 -B5 kina

# Network inspection
container exec <container-id> netstat -tuln
container exec <container-id> ip route show
```

### Emergency Recovery

```bash
# Stop all containers
container stop $(container list -q)

# Clean up resources
container system prune -a

# Reset kina state
rm -rf ~/.kina/clusters/*
rm -rf ~/.kina/cache/*

# Restart system services
sudo launchctl stop com.kina.cri-server
sudo launchctl start com.kina.cri-server
```

## Common Error Codes Reference

| Error Code | Description | Solution |
|------------|-------------|----------|
| `EXIT_001` | Virtualization framework unavailable | Install macOS 26+ beta |
| `EXIT_002` | Insufficient permissions | Add virtualization entitlements |
| `EXIT_003` | Container creation timeout | Increase timeout, check resources |
| `EXIT_004` | Image pull failure | Check registry authentication |
| `EXIT_005` | CRI server connection failed | Verify socket permissions |
| `EXIT_006` | Swift FFI binding error | Rebuild with correct Xcode version |
| `EXIT_007` | Network configuration error | Check DNS and routing |
| `EXIT_008` | Resource allocation failure | Free memory, reduce VM limits |

## Support Resources

### Documentation
- [Apple Container GitHub](https://github.com/apple/container)
- [Virtualization Framework Docs](https://developer.apple.com/documentation/virtualization)
- [Kubernetes CRI Specification](https://kubernetes.io/docs/concepts/architecture/cri/)

### Debugging Commands
```bash
# Complete system diagnostic
kina debug system-info > kina-system-report.txt

# CRI compliance test
kina debug cri-test --verbose

# Network connectivity test
kina debug network-test --cluster test-cluster
```