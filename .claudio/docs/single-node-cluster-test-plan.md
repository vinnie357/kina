# Single Node Cluster Integration Test Plan

**Project**: kina (Kubernetes in Apple Container)
**Test Plan Version**: 1.0
**Created**: 2025-01-20
**Test Type**: User Integration Testing (Real Environment)

> **Note**: This is an integration test plan for real user workflows with actual Apple Container runtime. Unit tests with mocks are handled separately in the project test suite (`kina-cli/tests/`).

## Overview

This integration test plan validates real-world user workflows with kina CLI against actual Apple Container runtime. Tests run as if performed by end users, requiring functional Apple Container environment.

**Test Philosophy**:
- **Integration Focus**: Test actual kina CLI binary with real Apple Container backend
- **User Perspective**: All tests simulate real user interactions and workflows
- **No Mocking**: Full stack testing with actual container runtime and Kubernetes components
- **Unit Test Separation**: Mock-based tests remain in `kina-cli/tests/` for development/CI

## Test Environment Requirements

### Prerequisites
- macOS 15.6+ with Apple Container support
- mise development environment configured
- Network connectivity for Kubernetes components

**Verification Commands**:
```bash
# Check Apple Container availability
mise run container-check

# Check Kubernetes tools (kubectl, kubectx, kubens)
mise run k8s-check

# Run basic CLI functionality tests
mise run test-cli
```

### Test Data Setup
- Test cluster configurations (minimal, standard, custom)
- Sample Kubernetes manifests for deployment testing
- Real container images for deployment validation

## Core Workflow Tests

### WF-001: Basic Cluster Lifecycle
**Objective**: Validate complete cluster create → use → destroy workflow

**Test Steps**:
1. **Cluster Creation**
   ```bash
   mise run kina create cluster test-cluster-001
   ```
   - Verify cluster appears in `mise run kina list`
   - Verify kubeconfig context created
   - Verify kubectl can connect: `kubectl cluster-info`

2. **Cluster Validation**
   ```bash
   kubectl get nodes
   kubectl get pods -A
   ```
   - Verify single node in Ready state
   - Verify system pods running (kube-system namespace)

3. **Basic Workload Deployment**
   ```bash
   kubectl create deployment nginx --image=nginx:latest
   kubectl expose deployment nginx --port=80 --target-port=80
   ```
   - Verify deployment succeeds
   - Verify pod reaches Running state
   - Verify service creation

4. **Cluster Cleanup**
   ```bash
   mise run kina delete cluster test-cluster-001
   ```
   - Verify cluster removed from `mise run kina list`
   - Verify kubeconfig context cleaned up
   - Verify Apple Container resources released

**Expected Results**:
- All commands succeed without errors
- Cluster fully functional for basic Kubernetes operations
- Clean teardown with no resource leaks

### WF-002: Cluster Configuration Options
**Objective**: Test various cluster configuration scenarios

**Test Variations**:
1. **Default Configuration**
   ```bash
   mise run kina create cluster default-test
   ```

2. **Custom Kubernetes Version**
   ```bash
   mise run kina create cluster version-test --kubernetes-version=v1.28.0
   ```

3. **Custom Cluster Name**
   ```bash
   mise run kina create cluster custom-name-test --name=my-dev-cluster
   ```

4. **Configuration File**
   ```bash
   mise run kina create cluster config-test --config=test-cluster-config.yaml
   ```

**Validation**: Each configuration creates functional cluster with specified parameters

### WF-003: Error Handling and Recovery
**Objective**: Validate error scenarios and recovery mechanisms

**Test Cases**:
1. **Duplicate Cluster Creation**
   - Create cluster → attempt to create same name
   - Expected: Error with clear message

2. **Invalid Cluster Names**
   - Test names with invalid characters
   - Expected: Validation error before creation

3. **Resource Constraints**
   - Test with limited system resources
   - Expected: Graceful failure or resource warnings

4. **Interrupted Operations**
   - Interrupt cluster creation mid-process
   - Verify cleanup and ability to retry

### WF-004: Apple Container Integration
**Objective**: Validate Apple Container-specific functionality

**Test Focus Areas**:
1. **Container Runtime Verification**
   ```bash
   kubectl describe nodes
   ```
   - Verify Apple Container as runtime
   - Check container runtime version

2. **Image Management**
   ```bash
   mise run kina load docker-image nginx:latest --cluster=test
   ```
   - Verify image loading into Apple Container
   - Test image availability in cluster

3. **Networking Validation**
   - Test pod-to-pod communication
   - Test service discovery
   - Test ingress connectivity (if supported)

## Integration Tests

### INT-001: Kubernetes Tools Compatibility
**Test kubectl Integration**:
```bash
# Context switching
kubectl config get-contexts
kubectl config use-context kina-test-cluster

# Cluster operations
kubectl apply -f test-manifests/
kubectl get all -A
kubectl logs -l app=test-app
```

**Test kubectx/kubens** (when available):
```bash
kubectx kina-test-cluster
kubens kube-system
```

### INT-002: Common Kubernetes Workflows
**Application Deployment Test**:
1. Deploy multi-tier application
2. Test service mesh capabilities (if supported)
3. Test persistent volumes (if supported)
4. Test config maps and secrets

**Ingress Controller Test** (if nginx-ingress supported):
```bash
kubectl apply -f nginx-ingress-controller.yaml
kubectl create ingress test-ingress --rule="test.local/*=test-service:80"
```

## Performance Tests

### PERF-001: Cluster Creation Time
**Metrics to Capture**:
- Time from `kina create` to cluster ready
- Resource utilization during creation
- Comparison baseline with kind (if available)

**Test Method**:
```bash
time mise run kina create cluster perf-test-001
kubectl wait --for=condition=Ready nodes --timeout=300s
```

### PERF-002: Resource Utilization
**Monitor During Tests**:
- Memory usage of Apple Container processes
- CPU utilization during cluster operations
- Storage usage for cluster data

## Automation Framework

### Integration Test Scripts
**integration-test-runner.sh**:
```bash
#!/bin/bash
# Real environment test execution
# Requires Apple Container runtime and built kina binary

set -e
echo "Starting kina integration tests..."

# Verify prerequisites using mise
mise run container-check || { echo "Apple Container not available"; exit 1; }
mise run k8s-check || { echo "Kubernetes tools not available"; exit 1; }
mise run test-cli || { echo "CLI functionality tests failed"; exit 1; }

# Execute real workflow tests
./test-workflows/basic-cluster-lifecycle.sh
./test-workflows/configuration-options.sh
./test-workflows/error-handling.sh
```

**integration-cleanup.sh**:
```bash
#!/bin/bash
# Clean up real clusters created during integration testing
mise run kina list | grep -E "test-cluster|integration-" | xargs -I {} mise run kina delete cluster {}
```

### CI Integration Note
- Integration tests run in dedicated environments with Apple Container
- Separate from unit test CI pipeline (which uses mocks)
- Manual execution or scheduled runs on macOS agents

## Test Execution Environment

### Real Environment Requirements
This test plan requires a fully functional environment:
- **Apple Container Runtime**: Must be installed and operational
- **kina CLI Binary**: Built and installed locally or in PATH
- **System Resources**: Sufficient memory/CPU for Kubernetes cluster
- **Network Access**: Required for pulling Kubernetes images and components

### Test Execution Approach
All tests execute against real systems:
- Use actual `kina` binary (not test framework)
- Create real Kubernetes clusters via Apple Container
- Deploy actual container workloads
- Measure real performance characteristics

## Test Data and Fixtures

### Sample Cluster Configurations
**minimal-cluster.yaml**:
```yaml
apiVersion: kina.dev/v1
kind: Cluster
metadata:
  name: minimal-test
spec:
  nodes:
  - role: control-plane
  kubernetesVersion: v1.28.0
```

### Sample Kubernetes Manifests
**test-deployment.yaml**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-app
spec:
  replicas: 1
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
        image: nginx:latest
        ports:
        - containerPort: 80
```

## Expected Results and Success Criteria

### Success Criteria
1. **Functional Completeness**: All core workflows complete successfully
2. **Performance Acceptance**: Cluster creation within acceptable time limits
3. **Reliability**: Tests pass consistently across multiple runs
4. **Integration**: Kubernetes tools work correctly with created clusters

### Failure Criteria
- Any core workflow test fails
- Performance significantly worse than kind baseline
- Kubernetes incompatibility issues
- Resource leaks or cleanup failures

## Test Reporting

### Test Results Format
- Pass/Fail status for each test case
- Performance metrics and trends
- Error logs and diagnostics
- Environment and configuration details

### Continuous Monitoring
- Automated daily test runs
- Performance regression alerts
- Test coverage reporting
- Integration test status dashboard

---

## Implementation Priority

1. **Phase 1**: Basic workflow tests (WF-001, WF-002)
2. **Phase 2**: Error handling and Apple Container integration tests
3. **Phase 3**: Performance testing and automation framework
4. **Phase 4**: CI integration and continuous monitoring

This test plan ensures comprehensive validation of single-node cluster workflows while supporting iterative development and continuous quality assurance.