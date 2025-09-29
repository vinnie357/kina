# Phase 4: Optimization - Performance, Reliability, and Production Readiness

**Phase Objectives**: Optimize cluster operations performance, implement comprehensive reliability patterns, establish production-grade testing and monitoring, and achieve security hardening based on KIND's operational patterns

## Phase Overview
This phase transforms KINA from a functional prototype into a production-ready system by implementing KIND's performance optimization strategies, comprehensive error handling patterns, extensive testing frameworks, and operational monitoring capabilities. Focus on achieving performance parity with KIND while leveraging Apple Container advantages.

## Key Deliverables
- Performance-optimized cluster operations with benchmarking against KIND
- Comprehensive error handling and recovery mechanisms with graceful degradation
- Complete test suite including unit, integration, and performance tests
- Security hardening with container security best practices
- Production monitoring and observability systems

## Task Breakdown

### Task 1: Performance Optimization and Benchmarking
**Objective**: Achieve performance parity with KIND while optimizing for Apple Container runtime characteristics
**Dependencies**: Phase 3 Tasks 1-6
**Acceptance Criteria**:
- Cluster creation time competitive with KIND (within 20% for equivalent configurations)
- Memory usage optimized for macOS environments
- Container operation latency minimized through Apple Container optimizations
- Comprehensive performance benchmarking suite implemented

**Implementation Notes**:
Based on KIND's performance optimization strategies:
```rust
pub struct PerformanceOptimizer {
    metrics_collector: MetricsCollector,
    cache_manager: CacheManager,
    operation_profiler: OperationProfiler,
}

impl PerformanceOptimizer {
    pub async fn optimize_cluster_creation(&self, config: &ClusterConfig) -> Result<OptimizationPlan, KinaError> {
        let baseline_metrics = self.measure_baseline_performance(config).await?;

        let optimizations = vec![
            self.optimize_image_operations().await?,
            self.optimize_container_creation().await?,
            self.optimize_network_setup().await?,
            self.optimize_kubernetes_bootstrap().await?,
        ];

        let optimization_plan = OptimizationPlan {
            baseline_metrics,
            optimizations,
            target_metrics: self.calculate_target_metrics(&baseline_metrics),
        };

        Ok(optimization_plan)
    }

    async fn optimize_image_operations(&self) -> Result<Optimization, KinaError> {
        Ok(Optimization::ImageOptimization {
            // Implement parallel image pulling
            parallel_pulls: true,
            // Use Apple Container's native layer caching
            layer_cache_optimization: true,
            // Pre-pull common images
            image_preloading: vec![
                "kindest/node:v1.28.0".to_string(),
                "kindest/node:v1.29.0".to_string(),
            ],
            // Implement image deduplication
            deduplication: true,
        })
    }

    async fn optimize_container_creation(&self) -> Result<Optimization, KinaError> {
        Ok(Optimization::ContainerOptimization {
            // Concurrent container creation (KIND pattern)
            concurrent_creation: true,
            max_concurrent: num_cpus::get().min(8),
            // Apple Container specific optimizations
            native_networking: true,
            optimized_volume_mounts: true,
        })
    }
}

pub struct CacheManager {
    image_cache: ImageCache,
    config_cache: ConfigCache,
    kubeconfig_cache: KubeconfigCache,
}

impl CacheManager {
    pub async fn optimize_caching_strategy(&self) -> Result<(), KinaError> {
        // Implement intelligent caching based on usage patterns
        self.image_cache.enable_predictive_caching().await?;

        // Cache kubeadm configurations for common Kubernetes versions
        self.config_cache.preload_common_configs().await?;

        // Implement cache warming for frequently used operations
        self.warm_caches().await?;

        Ok(())
    }

    async fn warm_caches(&self) -> Result<(), KinaError> {
        let common_images = [
            "kindest/node:v1.28.0",
            "kindest/node:v1.29.0",
            "registry.k8s.io/ingress-nginx/controller:v1.8.1",
        ];

        let warm_futures: Vec<_> = common_images.iter()
            .map(|image| self.image_cache.ensure_cached(image))
            .collect();

        futures::future::try_join_all(warm_futures).await?;
        Ok(())
    }
}

pub struct PerformanceBenchmark {
    operation_timers: HashMap<String, Vec<Duration>>,
    memory_tracker: MemoryTracker,
    kind_baseline: KindBenchmarkResults,
}

impl PerformanceBenchmark {
    pub async fn run_comprehensive_benchmark(&mut self) -> Result<BenchmarkReport, KinaError> {
        let test_configs = vec![
            self.single_node_config(),
            self.multi_node_config(),
            self.ha_control_plane_config(),
        ];

        let mut results = BenchmarkResults::new();

        for config in &test_configs {
            // Benchmark cluster creation
            let creation_time = self.benchmark_cluster_creation(config).await?;
            results.add_metric("cluster_creation", creation_time);

            // Benchmark kubectl operations
            let kubectl_latency = self.benchmark_kubectl_operations(config).await?;
            results.add_metric("kubectl_latency", kubectl_latency);

            // Benchmark cluster deletion
            let deletion_time = self.benchmark_cluster_deletion(config).await?;
            results.add_metric("cluster_deletion", deletion_time);

            // Measure memory usage
            let memory_usage = self.memory_tracker.peak_usage();
            results.add_metric("peak_memory", memory_usage);
        }

        self.generate_benchmark_report(results).await
    }

    async fn benchmark_cluster_creation(&self, config: &ClusterConfig) -> Result<Duration, KinaError> {
        let start = Instant::now();

        let provider = AppleContainerProvider::new().await?;
        let lifecycle = ClusterLifecycle::new(Arc::new(provider));

        lifecycle.create_cluster(&ClusterOptions {
            config: config.clone(),
            wait_for_ready: Duration::from_secs(300),
            retain: false,
            stop_before_kubernetes: false,
        }).await?;

        Ok(start.elapsed())
    }
}
```

**Deliverables**:
- PerformanceOptimizer with Apple Container-specific optimizations
- CacheManager with intelligent caching strategies
- PerformanceBenchmark suite with KIND comparison baselines
- Performance monitoring and alerting system

### Task 2: Comprehensive Error Handling and Recovery
**Objective**: Implement KIND's robust error handling patterns with Apple Container-specific recovery mechanisms
**Dependencies**: Phase 2-3 Tasks
**Acceptance Criteria**:
- All error scenarios handled gracefully with actionable user messages
- Retry logic implemented for transient Apple Container failures
- Partial failure scenarios support graceful degradation
- Resource cleanup guaranteed in all failure modes

**Implementation Notes**:
```rust
pub struct ErrorRecoverySystem {
    retry_strategies: HashMap<ErrorType, RetryStrategy>,
    cleanup_manager: CleanupManager,
    diagnostic_collector: DiagnosticCollector,
}

impl ErrorRecoverySystem {
    pub fn new() -> Self {
        let mut retry_strategies = HashMap::new();

        // Define retry strategies for different error types
        retry_strategies.insert(
            ErrorType::AppleContainerTransient,
            RetryStrategy::exponential_backoff(3, Duration::from_secs(1))
        );

        retry_strategies.insert(
            ErrorType::NetworkTimeout,
            RetryStrategy::fixed_interval(5, Duration::from_secs(2))
        );

        retry_strategies.insert(
            ErrorType::ImagePullFailure,
            RetryStrategy::exponential_backoff(3, Duration::from_secs(5))
        );

        Self {
            retry_strategies,
            cleanup_manager: CleanupManager::new(),
            diagnostic_collector: DiagnosticCollector::new(),
        }
    }

    pub async fn execute_with_recovery<F, T>(&self,
        operation: F,
        error_context: ErrorContext,
    ) -> Result<T, KinaError>
    where
        F: Fn() -> Pin<Box<dyn Future<Output = Result<T, KinaError>>>> + Send + Sync,
    {
        let retry_strategy = self.retry_strategies
            .get(&error_context.error_type)
            .unwrap_or(&RetryStrategy::no_retry());

        let mut attempts = 0;
        let max_attempts = retry_strategy.max_attempts;

        loop {
            attempts += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    // Collect diagnostics for troubleshooting
                    self.diagnostic_collector.collect_error_diagnostics(&error).await?;

                    if attempts >= max_attempts {
                        // Final attempt failed - perform cleanup and return error
                        self.cleanup_manager.cleanup_after_failure(&error_context).await?;

                        return Err(self.enhance_error_with_diagnostics(error).await?);
                    }

                    // Determine if error is recoverable
                    if !self.is_recoverable_error(&error) {
                        self.cleanup_manager.cleanup_after_failure(&error_context).await?;
                        return Err(error);
                    }

                    // Wait before retry
                    let delay = retry_strategy.calculate_delay(attempts);
                    tokio::time::sleep(delay).await;

                    tracing::warn!("Operation failed (attempt {}/{}), retrying in {:?}: {}",
                        attempts, max_attempts, delay, error);
                }
            }
        }
    }

    async fn enhance_error_with_diagnostics(&self, error: KinaError) -> Result<KinaError, KinaError> {
        let diagnostics = self.diagnostic_collector.generate_diagnostic_report().await?;

        Ok(match error {
            KinaError::ClusterCreationFailed { reason, .. } => {
                KinaError::ClusterCreationFailed {
                    reason,
                    suggestion: format!(
                        "Cluster creation failed. Diagnostics:\n{}\n\nTroubleshooting steps:\n{}",
                        diagnostics.summary,
                        diagnostics.troubleshooting_steps.join("\n")
                    ),
                }
            }
            KinaError::AppleContainerUnavailable { .. } => {
                KinaError::AppleContainerUnavailable {
                    suggestion: format!(
                        "Apple Container unavailable. Check:\n{}\n{}",
                        diagnostics.container_status,
                        "1. Verify macOS version (15.6+)\n2. Check Apple Container installation\n3. Restart Apple Container service"
                    ),
                }
            }
            _ => error,
        })
    }
}

pub struct CleanupManager {
    cleanup_strategies: HashMap<OperationType, CleanupStrategy>,
}

impl CleanupManager {
    pub async fn cleanup_after_failure(&self, context: &ErrorContext) -> Result<(), KinaError> {
        let strategy = self.cleanup_strategies
            .get(&context.operation_type)
            .unwrap_or(&CleanupStrategy::default());

        match strategy {
            CleanupStrategy::ClusterCreation { partial_cleanup } => {
                if *partial_cleanup {
                    self.cleanup_partial_cluster(&context.cluster_name).await?;
                } else {
                    self.cleanup_complete_cluster(&context.cluster_name).await?;
                }
            }
            CleanupStrategy::ImageBuild { remove_partial_layers } => {
                if *remove_partial_layers {
                    self.cleanup_partial_image_build(&context.build_id).await?;
                }
            }
            CleanupStrategy::NoCleanup => {
                tracing::debug!("No cleanup required for operation: {:?}", context.operation_type);
            }
        }

        Ok(())
    }

    async fn cleanup_partial_cluster(&self, cluster_name: &str) -> Result<(), KinaError> {
        tracing::info!("Cleaning up partial cluster: {}", cluster_name);

        let provider = AppleContainerProvider::new().await?;

        // Remove any created containers
        let nodes = provider.list_nodes(cluster_name).await.unwrap_or_default();
        if !nodes.is_empty() {
            provider.delete_nodes(&nodes).await?;
        }

        // Remove cluster network
        if let Ok(network) = provider.get_cluster_network(cluster_name).await {
            provider.delete_network(&network).await?;
        }

        tracing::info!("Partial cluster cleanup completed for: {}", cluster_name);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RetryStrategy {
    pub max_attempts: usize,
    pub delay_calculator: DelayCalculator,
}

impl RetryStrategy {
    pub fn exponential_backoff(max_attempts: usize, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            delay_calculator: DelayCalculator::ExponentialBackoff {
                initial_delay,
                max_delay: Duration::from_secs(30),
                multiplier: 2.0,
            },
        }
    }

    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        self.delay_calculator.calculate(attempt)
    }
}
```

**Deliverables**:
- ErrorRecoverySystem with intelligent retry and cleanup strategies
- CleanupManager ensuring resource cleanup in all failure scenarios
- Enhanced error reporting with diagnostic information and troubleshooting guidance
- Comprehensive error handling patterns for all major operations

### Task 3: Comprehensive Testing Framework
**Objective**: Implement extensive testing coverage including unit, integration, and end-to-end tests with KIND compatibility validation
**Dependencies**: Phase 2-3 implementations
**Acceptance Criteria**:
- Unit test coverage above 80% for all core components
- Integration tests validate Apple Container interactions
- End-to-end tests cover complete user workflows
- Performance regression tests prevent performance degradation

**Implementation Notes**:
```rust
// Unit testing framework with comprehensive coverage
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    use mockall::{automock, predicate::*};

    #[automock]
    pub trait MockableContainerProvider: ContainerProvider {}

    #[tokio::test]
    async fn test_cluster_lifecycle_creation() {
        let mut mock_provider = MockMockableContainerProvider::new();

        // Set up expectations
        mock_provider
            .expect_provision()
            .with(predicate::always())
            .times(1)
            .returning(|_| Ok(()));

        mock_provider
            .expect_list_nodes()
            .with(eq("test-cluster"))
            .times(1)
            .returning(|_| Ok(vec![]));

        let lifecycle = ClusterLifecycle::new(Arc::new(mock_provider));

        let config = ClusterConfig::default_with_name("test-cluster");
        let options = ClusterOptions {
            config,
            wait_for_ready: Duration::from_secs(1),
            retain: false,
            stop_before_kubernetes: true, // Skip K8s setup for unit test
        };

        let result = lifecycle.create_cluster(&options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_apple_container_provider_error_handling() {
        let provider = AppleContainerProvider::new().await.unwrap();

        // Test with invalid configuration
        let invalid_config = ClusterConfig {
            name: "".to_string(), // Invalid empty name
            nodes: vec![],
            ..Default::default()
        };

        let result = provider.provision(&invalid_config).await;
        assert!(matches!(result, Err(KinaError::ConfigValidationError { .. })));
    }
}

// Integration testing framework
pub struct IntegrationTestSuite {
    test_environment: TestEnvironment,
    apple_container_client: AppleContainerClient,
}

impl IntegrationTestSuite {
    pub async fn run_apple_container_integration_tests(&self) -> Result<TestResults, KinaError> {
        let mut results = TestResults::new();

        // Test 1: Apple Container availability
        results.add_test_result(
            "apple_container_availability",
            self.test_apple_container_availability().await
        );

        // Test 2: Container lifecycle operations
        results.add_test_result(
            "container_lifecycle",
            self.test_container_lifecycle_operations().await
        );

        // Test 3: Network management
        results.add_test_result(
            "network_management",
            self.test_network_management().await
        );

        // Test 4: Image operations
        results.add_test_result(
            "image_operations",
            self.test_image_operations().await
        );

        Ok(results)
    }

    async fn test_container_lifecycle_operations(&self) -> Result<(), KinaError> {
        let provider = AppleContainerProvider::new().await?;
        let test_cluster = "integration-test-cluster";

        // Create test cluster configuration
        let config = ClusterConfig {
            name: test_cluster.to_string(),
            nodes: vec![
                NodeConfig {
                    role: NodeRole::ControlPlane,
                    image: Some("kindest/node:v1.28.0".to_string()),
                    ..Default::default()
                }
            ],
            ..Default::default()
        };

        // Test container creation
        provider.provision(&config).await?;

        // Verify containers exist
        let nodes = provider.list_nodes(test_cluster).await?;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].role, NodeRole::ControlPlane);

        // Test container deletion
        provider.delete_nodes(&nodes).await?;

        // Verify cleanup
        let remaining_nodes = provider.list_nodes(test_cluster).await?;
        assert!(remaining_nodes.is_empty());

        Ok(())
    }
}

// End-to-end testing framework
pub struct E2ETestSuite {
    kina_binary: PathBuf,
    test_workspace: TempDir,
}

impl E2ETestSuite {
    pub async fn run_complete_workflow_tests(&self) -> Result<E2EResults, KinaError> {
        let mut results = E2EResults::new();

        // Test complete cluster lifecycle
        results.add_result(
            "complete_cluster_lifecycle",
            self.test_complete_cluster_lifecycle().await
        );

        // Test KIND compatibility
        results.add_result(
            "kind_compatibility",
            self.test_kind_compatibility().await
        );

        // Test Kubernetes tool integration
        results.add_result(
            "kubernetes_tools_integration",
            self.test_kubernetes_tools_integration().await
        );

        Ok(results)
    }

    async fn test_complete_cluster_lifecycle(&self) -> Result<(), KinaError> {
        let cluster_name = "e2e-test-cluster";

        // Step 1: Create cluster
        let create_output = Command::new(&self.kina_binary)
            .args(&["create", "cluster", "--name", cluster_name, "--wait", "300s"])
            .output()
            .await?;

        assert!(create_output.status.success(),
            "Cluster creation failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        // Step 2: Verify cluster is functional
        let kubectl_output = Command::new("kubectl")
            .args(&["get", "nodes", "--context", &format!("kina-{}", cluster_name)])
            .output()
            .await?;

        assert!(kubectl_output.status.success(),
            "kubectl failed: {}",
            String::from_utf8_lossy(&kubectl_output.stderr)
        );

        // Step 3: Test pod deployment
        let pod_yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
  - name: test-container
    image: nginx:alpine
    ports:
    - containerPort: 80
"#;

        std::fs::write(self.test_workspace.path().join("test-pod.yaml"), pod_yaml)?;

        let apply_output = Command::new("kubectl")
            .args(&["apply", "-f", "test-pod.yaml", "--context", &format!("kina-{}", cluster_name)])
            .current_dir(&self.test_workspace.path())
            .output()
            .await?;

        assert!(apply_output.status.success());

        // Step 4: Wait for pod to be ready
        let wait_output = Command::new("kubectl")
            .args(&["wait", "--for=condition=Ready", "pod/test-pod", "--timeout=60s", "--context", &format!("kina-{}", cluster_name)])
            .output()
            .await?;

        assert!(wait_output.status.success());

        // Step 5: Clean up cluster
        let delete_output = Command::new(&self.kina_binary)
            .args(&["delete", "cluster", "--name", cluster_name])
            .output()
            .await?;

        assert!(delete_output.status.success());

        Ok(())
    }
}

// Performance regression testing
pub struct PerformanceRegressionTests {
    baseline_metrics: BaselineMetrics,
    current_metrics: CurrentMetrics,
}

impl PerformanceRegressionTests {
    pub async fn run_regression_tests(&self) -> Result<RegressionResults, KinaError> {
        let current_benchmark = self.run_performance_benchmark().await?;
        let regression_analysis = self.analyze_regression(&current_benchmark).await?;

        Ok(RegressionResults {
            current_metrics: current_benchmark,
            baseline_comparison: regression_analysis,
            regression_threshold_exceeded: regression_analysis.has_regressions(),
        })
    }

    async fn analyze_regression(&self, current: &BenchmarkResults) -> Result<RegressionAnalysis, KinaError> {
        let mut analysis = RegressionAnalysis::new();

        for (metric_name, current_value) in &current.metrics {
            if let Some(baseline_value) = self.baseline_metrics.get(metric_name) {
                let regression_percentage = ((current_value - baseline_value) / baseline_value) * 100.0;

                if regression_percentage > 10.0 { // 10% regression threshold
                    analysis.add_regression(Regression {
                        metric: metric_name.clone(),
                        baseline: *baseline_value,
                        current: *current_value,
                        regression_percentage,
                    });
                }
            }
        }

        Ok(analysis)
    }
}
```

**Deliverables**:
- Comprehensive unit test suite with >80% coverage
- Integration test framework validating Apple Container interactions
- End-to-end test suite covering complete user workflows
- Performance regression testing preventing performance degradation

### Task 4: Security Hardening and Container Security
**Objective**: Implement security best practices for Apple Container integration and Kubernetes cluster security
**Dependencies**: Phase 2-3 implementations
**Acceptance Criteria**:
- Container security policies implemented following best practices
- Secret and credential management secure
- Security scanning integrated for container images
- Security documentation and guidelines complete

**Implementation Notes**:
```rust
pub struct SecurityHardeningSystem {
    container_security: ContainerSecurityManager,
    credential_manager: CredentialManager,
    image_scanner: ImageScanner,
    security_policies: SecurityPolicyManager,
}

impl SecurityHardeningSystem {
    pub async fn apply_security_hardening(&self, cluster_config: &ClusterConfig) -> Result<(), KinaError> {
        // Apply container security policies
        self.container_security.apply_security_policies(cluster_config).await?;

        // Secure credential handling
        self.credential_manager.secure_cluster_credentials(cluster_config).await?;

        // Scan container images for vulnerabilities
        self.image_scanner.scan_cluster_images(cluster_config).await?;

        // Apply Kubernetes security policies
        self.security_policies.apply_kubernetes_policies(cluster_config).await?;

        Ok(())
    }
}

pub struct ContainerSecurityManager {
    apple_container_runtime: AppleContainerRuntime,
}

impl ContainerSecurityManager {
    pub async fn apply_security_policies(&self, config: &ClusterConfig) -> Result<(), KinaError> {
        for node_config in &config.nodes {
            // Apply security context constraints
            self.apply_security_context(&node_config).await?;

            // Configure AppArmor/SELinux profiles if available
            self.configure_security_profiles(&node_config).await?;

            // Set resource limits and quotas
            self.configure_resource_limits(&node_config).await?;
        }

        Ok(())
    }

    async fn apply_security_context(&self, node_config: &NodeConfig) -> Result<(), KinaError> {
        let security_context = ContainerSecurityContext {
            // Run as non-root when possible (except for systemd requirements)
            run_as_non_root: false, // Kubernetes nodes require root for systemd
            read_only_root_filesystem: false, // Kubernetes needs writable filesystem

            // Drop unnecessary capabilities
            drop_capabilities: vec![
                "NET_RAW".to_string(),
                "SYS_MODULE".to_string(),
                "SYS_TIME".to_string(),
            ],

            // Add only required capabilities
            add_capabilities: vec![
                "SYS_ADMIN".to_string(), // Required for systemd
                "NET_ADMIN".to_string(), // Required for networking
            ],

            // Prevent privilege escalation
            allow_privilege_escalation: false,
        };

        self.apple_container_runtime.apply_security_context(
            &node_config.name,
            security_context
        ).await?;

        Ok(())
    }
}

pub struct CredentialManager {
    keychain_service: KeychainService,
    encrypted_storage: EncryptedStorage,
}

impl CredentialManager {
    pub async fn secure_cluster_credentials(&self, config: &ClusterConfig) -> Result<(), KinaError> {
        // Generate and store cluster certificates securely
        let cluster_certs = self.generate_cluster_certificates(config).await?;
        self.store_certificates_securely(&cluster_certs).await?;

        // Secure kubeconfig storage
        let kubeconfig = self.generate_kubeconfig(config).await?;
        self.store_kubeconfig_securely(&config.name, &kubeconfig).await?;

        // Generate and store service account tokens
        let service_tokens = self.generate_service_account_tokens(config).await?;
        self.store_tokens_securely(&service_tokens).await?;

        Ok(())
    }

    async fn store_certificates_securely(&self, certs: &ClusterCertificates) -> Result<(), KinaError> {
        // Use macOS Keychain for certificate storage
        self.keychain_service.store_certificate(
            &certs.ca_cert,
            &format!("kina-ca-{}", certs.cluster_name)
        ).await?;

        self.keychain_service.store_private_key(
            &certs.ca_key,
            &format!("kina-ca-key-{}", certs.cluster_name)
        ).await?;

        Ok(())
    }
}

pub struct ImageScanner {
    vulnerability_database: VulnerabilityDatabase,
    scanning_engine: ScanningEngine,
}

impl ImageScanner {
    pub async fn scan_cluster_images(&self, config: &ClusterConfig) -> Result<ScanResults, KinaError> {
        let mut scan_results = ScanResults::new();

        for node_config in &config.nodes {
            if let Some(image) = &node_config.image {
                let image_scan = self.scan_image(image).await?;
                scan_results.add_image_scan(image.clone(), image_scan);
            }
        }

        // Fail if critical vulnerabilities found
        if scan_results.has_critical_vulnerabilities() {
            return Err(KinaError::SecurityViolation {
                reason: "Critical vulnerabilities found in container images".to_string(),
                details: scan_results.format_critical_vulnerabilities(),
            });
        }

        Ok(scan_results)
    }

    async fn scan_image(&self, image: &str) -> Result<ImageScanResult, KinaError> {
        // Update vulnerability database
        self.vulnerability_database.update().await?;

        // Perform image scan
        let scan_result = self.scanning_engine.scan(image).await?;

        // Generate security report
        let security_report = SecurityReport {
            image: image.to_string(),
            vulnerabilities: scan_result.vulnerabilities,
            security_score: scan_result.calculate_security_score(),
            recommendations: scan_result.generate_recommendations(),
        };

        Ok(ImageScanResult {
            image: image.to_string(),
            scan_timestamp: chrono::Utc::now(),
            security_report,
        })
    }
}

pub struct SecurityPolicyManager;

impl SecurityPolicyManager {
    pub async fn apply_kubernetes_policies(&self, config: &ClusterConfig) -> Result<(), KinaError> {
        // Apply Pod Security Standards
        self.apply_pod_security_standards(config).await?;

        // Configure Network Policies
        self.apply_network_policies(config).await?;

        // Set up RBAC policies
        self.configure_rbac_policies(config).await?;

        Ok(())
    }

    async fn apply_pod_security_standards(&self, config: &ClusterConfig) -> Result<(), KinaError> {
        let pod_security_policy = r#"
apiVersion: v1
kind: Namespace
metadata:
  name: kube-system
  labels:
    pod-security.kubernetes.io/enforce: privileged
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
---
apiVersion: v1
kind: Namespace
metadata:
  name: default
  labels:
    pod-security.kubernetes.io/enforce: baseline
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
"#;

        // Apply policy using kubectl
        self.apply_kubernetes_manifest(&config.name, pod_security_policy).await?;

        Ok(())
    }

    async fn apply_kubernetes_manifest(&self, cluster_name: &str, manifest: &str) -> Result<(), KinaError> {
        let provider = AppleContainerProvider::new().await?;
        let control_plane_node = provider.find_control_plane_node(cluster_name).await?;

        let apply_cmd = ["kubectl", "apply", "-f", "-"];

        let result = provider.exec_in_container_with_stdin(
            &control_plane_node.container_id,
            &apply_cmd,
            manifest,
        ).await?;

        if !result.success {
            return Err(KinaError::SecurityPolicyFailed {
                reason: "Failed to apply Kubernetes security policy".to_string(),
                stderr: result.stderr,
            });
        }

        Ok(())
    }
}
```

**Deliverables**:
- SecurityHardeningSystem with comprehensive container and Kubernetes security
- CredentialManager using macOS Keychain for secure credential storage
- ImageScanner with vulnerability detection and reporting
- Security policy templates and automated application system

### Task 5: Production Monitoring and Observability
**Objective**: Implement comprehensive monitoring, logging, and observability for production operations
**Dependencies**: Phase 2-3 implementations
**Acceptance Criteria**:
- Structured logging implemented throughout the application
- Metrics collection for all major operations
- Health checks for clusters and system components
- Diagnostic tools for troubleshooting operational issues

**Implementation Notes**:
```rust
pub struct ObservabilitySystem {
    metrics_collector: MetricsCollector,
    logging_system: StructuredLogging,
    health_checker: HealthChecker,
    diagnostic_tools: DiagnosticTools,
}

impl ObservabilitySystem {
    pub async fn initialize(&self) -> Result<(), KinaError> {
        self.metrics_collector.start_collection().await?;
        self.logging_system.configure_structured_logging().await?;
        self.health_checker.start_health_monitoring().await?;

        Ok(())
    }
}

pub struct MetricsCollector {
    metrics_registry: MetricsRegistry,
    exporters: Vec<Box<dyn MetricsExporter>>,
}

impl MetricsCollector {
    pub fn record_cluster_operation(&self, operation: &str, duration: Duration, success: bool) {
        let labels = vec![
            ("operation", operation),
            ("success", &success.to_string()),
        ];

        self.metrics_registry.histogram("kina_operation_duration_seconds")
            .with_labels(&labels)
            .observe(duration.as_secs_f64());

        self.metrics_registry.counter("kina_operations_total")
            .with_labels(&labels)
            .inc();
    }

    pub fn record_cluster_count(&self, count: usize) {
        self.metrics_registry.gauge("kina_active_clusters")
            .set(count as f64);
    }

    pub fn record_apple_container_operation(&self, operation: &str, latency: Duration) {
        self.metrics_registry.histogram("kina_apple_container_operation_duration_seconds")
            .with_labels(&[("operation", operation)])
            .observe(latency.as_secs_f64());
    }
}

pub struct HealthChecker {
    health_checks: Vec<Box<dyn HealthCheck>>,
    health_status: Arc<RwLock<HealthStatus>>,
}

impl HealthChecker {
    pub async fn check_system_health(&self) -> Result<SystemHealthReport, KinaError> {
        let mut health_report = SystemHealthReport::new();

        for health_check in &self.health_checks {
            let check_result = health_check.perform_check().await;
            health_report.add_check_result(health_check.name(), check_result);
        }

        // Update overall health status
        let overall_status = if health_report.has_critical_failures() {
            HealthStatus::Critical
        } else if health_report.has_warnings() {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        *self.health_status.write().await = overall_status;

        Ok(health_report)
    }
}

#[async_trait]
pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    async fn perform_check(&self) -> HealthCheckResult;
}

pub struct AppleContainerHealthCheck {
    runtime: AppleContainerRuntime,
}

#[async_trait]
impl HealthCheck for AppleContainerHealthCheck {
    fn name(&self) -> &str {
        "apple_container_runtime"
    }

    async fn perform_check(&self) -> HealthCheckResult {
        match self.runtime.health_check().await {
            Ok(_) => HealthCheckResult::healthy(),
            Err(e) => HealthCheckResult::unhealthy(format!("Apple Container runtime unavailable: {}", e)),
        }
    }
}

pub struct ClusterHealthCheck {
    cluster_name: String,
    provider: Arc<dyn ContainerProvider>,
}

#[async_trait]
impl HealthCheck for ClusterHealthCheck {
    fn name(&self) -> &str {
        "cluster_health"
    }

    async fn perform_check(&self) -> HealthCheckResult {
        // Check if cluster nodes are running
        match self.provider.list_nodes(&self.cluster_name).await {
            Ok(nodes) => {
                if nodes.is_empty() {
                    HealthCheckResult::warning("No nodes found in cluster".to_string())
                } else {
                    // Check individual node health
                    let unhealthy_nodes: Vec<_> = nodes.iter()
                        .filter(|node| !node.is_healthy())
                        .collect();

                    if unhealthy_nodes.is_empty() {
                        HealthCheckResult::healthy()
                    } else {
                        HealthCheckResult::warning(format!(
                            "{} unhealthy nodes: {:?}",
                            unhealthy_nodes.len(),
                            unhealthy_nodes.iter().map(|n| &n.name).collect::<Vec<_>>()
                        ))
                    }
                }
            }
            Err(e) => HealthCheckResult::unhealthy(format!("Failed to check cluster health: {}", e)),
        }
    }
}

pub struct DiagnosticTools {
    system_info_collector: SystemInfoCollector,
    log_analyzer: LogAnalyzer,
    configuration_validator: ConfigurationValidator,
}

impl DiagnosticTools {
    pub async fn generate_diagnostic_report(&self, cluster_name: &str) -> Result<DiagnosticReport, KinaError> {
        let system_info = self.system_info_collector.collect_system_info().await?;
        let log_analysis = self.log_analyzer.analyze_recent_logs().await?;
        let config_validation = self.configuration_validator.validate_cluster_config(cluster_name).await?;

        Ok(DiagnosticReport {
            timestamp: chrono::Utc::now(),
            cluster_name: cluster_name.to_string(),
            system_info,
            log_analysis,
            config_validation,
            troubleshooting_recommendations: self.generate_troubleshooting_recommendations(&log_analysis),
        })
    }

    fn generate_troubleshooting_recommendations(&self, log_analysis: &LogAnalysis) -> Vec<TroubleshootingRecommendation> {
        let mut recommendations = Vec::new();

        if log_analysis.has_apple_container_errors() {
            recommendations.push(TroubleshootingRecommendation {
                category: "Apple Container".to_string(),
                severity: Severity::High,
                description: "Apple Container errors detected".to_string(),
                actions: vec![
                    "Check Apple Container service status".to_string(),
                    "Verify macOS version compatibility (15.6+)".to_string(),
                    "Restart Apple Container service if necessary".to_string(),
                ],
            });
        }

        if log_analysis.has_network_errors() {
            recommendations.push(TroubleshootingRecommendation {
                category: "Networking".to_string(),
                severity: Severity::Medium,
                description: "Network connectivity issues detected".to_string(),
                actions: vec![
                    "Check network connectivity".to_string(),
                    "Verify container network configuration".to_string(),
                    "Check firewall settings".to_string(),
                ],
            });
        }

        recommendations
    }
}
```

**Deliverables**:
- ObservabilitySystem with comprehensive metrics collection and health monitoring
- HealthChecker with system and cluster health validation
- DiagnosticTools for automated troubleshooting and problem identification
- Structured logging and metrics export for production monitoring

### Task 6: Documentation and User Experience
**Objective**: Create comprehensive documentation, user guides, and developer resources
**Dependencies**: All previous phase implementations
**Acceptance Criteria**:
- Complete user documentation with installation and usage guides
- Troubleshooting and FAQ documentation based on testing findings
- Developer documentation for contributing and extending KINA
- Performance benchmarking results and recommendations

**Implementation Notes**:
```rust
pub struct DocumentationGenerator {
    markdown_generator: MarkdownGenerator,
    example_generator: ExampleGenerator,
    benchmark_reporter: BenchmarkReporter,
}

impl DocumentationGenerator {
    pub async fn generate_complete_documentation(&self) -> Result<DocumentationSet, KinaError> {
        let docs = DocumentationSet {
            user_guide: self.generate_user_guide().await?,
            installation_guide: self.generate_installation_guide().await?,
            troubleshooting_guide: self.generate_troubleshooting_guide().await?,
            performance_guide: self.generate_performance_guide().await?,
            developer_guide: self.generate_developer_guide().await?,
            api_reference: self.generate_api_reference().await?,
            examples: self.example_generator.generate_all_examples().await?,
        };

        Ok(docs)
    }

    async fn generate_user_guide(&self) -> Result<UserGuide, KinaError> {
        let guide = UserGuide {
            introduction: self.generate_introduction_section().await?,
            quick_start: self.generate_quick_start_section().await?,
            command_reference: self.generate_command_reference().await?,
            configuration_guide: self.generate_configuration_guide().await?,
            advanced_usage: self.generate_advanced_usage_section().await?,
        };

        Ok(guide)
    }

    async fn generate_troubleshooting_guide(&self) -> Result<TroubleshootingGuide, KinaError> {
        let common_issues = vec![
            TroubleshootingSection {
                title: "Apple Container Not Available".to_string(),
                description: "kina reports Apple Container is not available or not running".to_string(),
                symptoms: vec![
                    "Error: Apple Container not found or not running".to_string(),
                    "Cluster creation fails immediately".to_string(),
                ],
                solutions: vec![
                    "Verify macOS version is 15.6 or later".to_string(),
                    "Check if Apple Container is installed".to_string(),
                    "Restart Apple Container service".to_string(),
                ],
                related_commands: vec![
                    "kina version".to_string(),
                    "system_profiler SPSoftwareDataType".to_string(),
                ],
            },
            TroubleshootingSection {
                title: "Cluster Creation Hangs".to_string(),
                description: "Cluster creation starts but never completes".to_string(),
                symptoms: vec![
                    "kina create cluster hangs for more than 10 minutes".to_string(),
                    "Containers are created but Kubernetes doesn't start".to_string(),
                ],
                solutions: vec![
                    "Check network connectivity for image pulls".to_string(),
                    "Verify sufficient disk space".to_string(),
                    "Try with --verbose flag for detailed output".to_string(),
                    "Check system resources (CPU, Memory)".to_string(),
                ],
                related_commands: vec![
                    "kina create cluster --verbose".to_string(),
                    "kina get clusters".to_string(),
                    "df -h".to_string(),
                ],
            },
        ];

        Ok(TroubleshootingGuide { sections: common_issues })
    }

    async fn generate_performance_guide(&self) -> Result<PerformanceGuide, KinaError> {
        let benchmark_results = self.benchmark_reporter.generate_benchmark_report().await?;

        let performance_recommendations = vec![
            PerformanceRecommendation {
                category: "Image Management".to_string(),
                recommendation: "Pre-pull common node images to reduce cluster creation time".to_string(),
                command: "kina build node-image --kubernetes-version v1.28.0".to_string(),
                expected_improvement: "30-50% faster cluster creation".to_string(),
            },
            PerformanceRecommendation {
                category: "Resource Allocation".to_string(),
                recommendation: "Allocate sufficient resources for Apple Container".to_string(),
                details: "Ensure at least 4GB RAM and 20GB disk space available".to_string(),
                expected_improvement: "Prevents resource contention issues".to_string(),
            },
        ];

        Ok(PerformanceGuide {
            benchmark_results,
            recommendations: performance_recommendations,
            tuning_guide: self.generate_tuning_guide().await?,
        })
    }
}

pub struct ExampleGenerator;

impl ExampleGenerator {
    pub async fn generate_all_examples(&self) -> Result<Vec<Example>, KinaError> {
        let examples = vec![
            self.generate_basic_cluster_example().await?,
            self.generate_multi_node_cluster_example().await?,
            self.generate_custom_configuration_example().await?,
            self.generate_ingress_example().await?,
            self.generate_ci_cd_example().await?,
        ];

        Ok(examples)
    }

    async fn generate_basic_cluster_example(&self) -> Result<Example, KinaError> {
        Ok(Example {
            title: "Basic Single-Node Cluster".to_string(),
            description: "Create a simple single-node Kubernetes cluster for local development".to_string(),
            commands: vec![
                "# Create a cluster".to_string(),
                "kina create cluster --name my-cluster".to_string(),
                "".to_string(),
                "# Verify cluster is running".to_string(),
                "kubectl get nodes".to_string(),
                "".to_string(),
                "# Deploy a sample application".to_string(),
                "kubectl create deployment nginx --image=nginx".to_string(),
                "kubectl expose deployment nginx --port=80 --type=NodePort".to_string(),
                "".to_string(),
                "# Clean up".to_string(),
                "kina delete cluster --name my-cluster".to_string(),
            ],
            expected_output: Some("node/my-cluster-control-plane   Ready    control-plane   1m    v1.28.0".to_string()),
            notes: vec![
                "Default cluster uses Kubernetes v1.28.0".to_string(),
                "Cluster will be available via kubectl automatically".to_string(),
            ],
        })
    }
}
```

**Deliverables**:
- DocumentationGenerator creating comprehensive user and developer documentation
- TroubleshootingGuide with common issues and solutions
- PerformanceGuide with benchmarking results and optimization recommendations
- Complete example library covering all major use cases

## Success Criteria
- Performance metrics match or exceed KIND baselines for equivalent operations
- All error scenarios handled gracefully with actionable user feedback
- Test suite achieves >80% coverage with comprehensive integration and e2e testing
- Security hardening passes security review without critical findings
- Documentation provides complete coverage for users and developers

## Critical Dependencies
- **Phase 2-3 completion**: All core functionality must be implemented and stable
- **Apple Container performance characteristics**: Understanding of platform-specific optimizations
- **Security review resources**: Access to security scanning tools and expertise

## Risk Mitigation
- **Performance targets**: Establish realistic benchmarks based on Apple Container capabilities
- **Testing complexity**: Prioritize high-impact test scenarios with incremental coverage improvement
- **Security requirements**: Implement security hardening incrementally with regular reviews

## Integration Notes
- Foundation for Phase 5 production release and distribution
- Performance optimizations maintain compatibility with existing features
- Security hardening follows industry best practices while accommodating Kubernetes requirements

**Phase Completion Gate**: All optimizations, testing, and security measures must be complete and validated before production release preparation