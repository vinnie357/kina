# Apple Container Security Analysis

**Focus**: Security model, compliance considerations, and implementation patterns

## Apple Container Security Model

### Hardware-Level Isolation Benefits

**VM-per-Container Architecture:**
```
Traditional Container Security:
Host Kernel → Namespace Isolation → Container Process
└─ Shared kernel vulnerabilities
└─ Container escape risks
└─ Resource contention

Apple Container Security:
Host macOS → Virtualization.framework → Dedicated Linux VM → Container Process
└─ Hardware-enforced isolation
└─ VM-level containment
└─ Dedicated resource allocation
```

**Security Advantages:**
- **Complete Kernel Isolation**: Each container runs in dedicated VM with separate kernel
- **Hardware-Enforced Memory Boundaries**: VM isolation prevents memory-based attacks
- **Per-Container Attack Surface**: Security breaches contained within individual VMs
- **Eliminates Namespace Escape**: No shared kernel namespace vulnerabilities
- **Resource Isolation**: Prevents "noisy neighbor" attacks and resource exhaustion

### macOS System Integration Requirements

#### Required System Entitlements

```xml
<!-- App entitlements for Virtualization framework access -->
<key>com.apple.security.virtualization</key>
<true/>

<!-- Network access for container networking -->
<key>com.apple.security.network.client</key>
<true/>

<!-- File system access for container image storage -->
<key>com.apple.security.files.user-selected.read-write</key>
<true/>

<!-- Hardened runtime compatibility -->
<key>com.apple.security.cs.allow-jit</key>
<true/>

<!-- Code signing requirements -->
<key>com.apple.security.cs.allow-unsigned-executable-memory</key>
<false/>
```

#### System Requirements and Validation

```rust
// kina-cli/src/core/security/system_validation.rs
use std::process::Command;

pub struct SystemSecurityValidator;

impl SystemSecurityValidator {
    pub fn validate_system_requirements() -> Result<(), SecurityError> {
        // Check macOS version
        if !Self::is_macos_supported()? {
            return Err(SecurityError::UnsupportedOS);
        }

        // Verify Apple Silicon
        if !Self::is_apple_silicon()? {
            return Err(SecurityError::UnsupportedArchitecture);
        }

        // Check virtualization entitlements
        if !Self::has_virtualization_entitlement()? {
            return Err(SecurityError::MissingEntitlements);
        }

        // Verify code signing
        if !Self::is_properly_signed()? {
            return Err(SecurityError::InvalidCodeSignature);
        }

        Ok(())
    }

    fn is_macos_supported() -> Result<bool, SecurityError> {
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        let version = String::from_utf8(output.stdout)
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        // Parse version and check for macOS 26+
        let version_parts: Vec<&str> = version.trim().split('.').collect();
        let major_version: u32 = version_parts[0].parse()
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        Ok(major_version >= 26)
    }

    fn is_apple_silicon() -> Result<bool, SecurityError> {
        let output = Command::new("uname")
            .arg("-m")
            .output()
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        let arch = String::from_utf8(output.stdout)
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        Ok(arch.trim() == "arm64")
    }

    fn has_virtualization_entitlement() -> Result<bool, SecurityError> {
        // Check current process entitlements
        let output = Command::new("codesign")
            .args(&["-d", "--entitlements", "-", "/proc/self/exe"])
            .output()
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        let entitlements = String::from_utf8(output.stdout)
            .map_err(|e| SecurityError::SystemCheckFailed(e.to_string()))?;

        Ok(entitlements.contains("com.apple.security.virtualization"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Unsupported operating system")]
    UnsupportedOS,

    #[error("Apple Silicon architecture required")]
    UnsupportedArchitecture,

    #[error("Missing virtualization entitlements")]
    MissingEntitlements,

    #[error("Invalid code signature")]
    InvalidCodeSignature,

    #[error("System security check failed: {0}")]
    SystemCheckFailed(String),
}
```

## Security Implementation Patterns

### Container Security Context

```rust
// kina-cli/src/core/security/context.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub container_isolation: IsolationLevel,
    pub resource_constraints: ResourceConstraints,
    pub network_policies: NetworkPolicies,
    pub volume_access: VolumeAccessControl,
    pub privilege_escalation: PrivilegeEscalationPolicy,
}

#[derive(Debug, Clone)]
pub enum IsolationLevel {
    HardwareVirtualization,  // Apple Container default
    ProcessNamespace,        // Traditional containers (not supported)
}

#[derive(Debug, Clone)]
pub struct ResourceConstraints {
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<u64>,
    pub disk_io_limit: Option<u64>,
    pub network_bandwidth_limit: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct NetworkPolicies {
    pub ingress_rules: Vec<NetworkRule>,
    pub egress_rules: Vec<NetworkRule>,
    pub dns_policy: DnsPolicy,
    pub isolation_mode: NetworkIsolation,
}

#[derive(Debug, Clone)]
pub enum NetworkIsolation {
    Complete,        // No inter-container communication
    ClusterOnly,     // Only Kubernetes cluster communication
    HostAccessible,  // Accessible from host network
}

#[derive(Debug, Clone)]
pub struct NetworkRule {
    pub protocol: Protocol,
    pub ports: Vec<u16>,
    pub sources: Vec<NetworkSource>,
    pub action: NetworkAction,
}

#[derive(Debug, Clone)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
}

#[derive(Debug, Clone)]
pub enum NetworkSource {
    CIDR(String),
    PodSelector(HashMap<String, String>),
    NamespaceSelector(HashMap<String, String>),
}

#[derive(Debug, Clone)]
pub enum NetworkAction {
    Allow,
    Deny,
    Log,
}

impl SecurityContext {
    pub fn validate_container_security(&self, config: &ContainerConfig) -> Result<(), SecurityError> {
        // Validate privileged container requirements
        if config.privileged && !self.is_privileged_allowed() {
            return Err(SecurityError::PrivilegedNotAllowed);
        }

        // Validate resource constraints
        self.validate_resource_limits(&config.resources)?;

        // Validate volume mounts for security violations
        self.validate_volume_mounts(&config.volumes)?;

        // Validate network access policies
        self.validate_network_access(&config.network)?;

        Ok(())
    }

    fn is_privileged_allowed(&self) -> bool {
        match self.privilege_escalation {
            PrivilegeEscalationPolicy::Allowed => true,
            PrivilegeEscalationPolicy::RequireApproval => {
                // Check for admin approval or security policy
                self.check_privilege_approval()
            }
            PrivilegeEscalationPolicy::Denied => false,
        }
    }

    fn validate_resource_limits(&self, resources: &ResourceConstraints) -> Result<(), SecurityError> {
        // Enforce maximum resource limits to prevent DoS
        if let Some(cpu) = resources.cpu_limit {
            if cpu > 8.0 {  // Maximum 8 CPU cores
                return Err(SecurityError::ExcessiveResourceRequest("CPU"));
            }
        }

        if let Some(memory) = resources.memory_limit {
            if memory > 16 * 1024 * 1024 * 1024 {  // Maximum 16GB
                return Err(SecurityError::ExcessiveResourceRequest("Memory"));
            }
        }

        Ok(())
    }

    fn validate_volume_mounts(&self, volumes: &[VolumeMount]) -> Result<(), SecurityError> {
        for volume in volumes {
            // Check for sensitive system paths
            if self.is_sensitive_path(&volume.target) {
                return Err(SecurityError::SensitivePathAccess(volume.target.clone()));
            }

            // Validate host path mounts
            if self.is_host_path_mount(volume) {
                self.validate_host_path_security(volume)?;
            }
        }

        Ok(())
    }

    fn is_sensitive_path(&self, path: &str) -> bool {
        let sensitive_paths = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/ssh",
            "/var/run/docker.sock",
            "/proc",
            "/sys",
            "/dev",
        ];

        sensitive_paths.iter().any(|&sensitive| path.starts_with(sensitive))
    }

    fn validate_host_path_security(&self, volume: &VolumeMount) -> Result<(), SecurityError> {
        // Ensure host paths are not writable system directories
        let dangerous_host_paths = [
            "/",
            "/usr",
            "/etc",
            "/var",
            "/sys",
            "/proc",
        ];

        if dangerous_host_paths.iter().any(|&path| volume.source.starts_with(path)) && !volume.read_only {
            return Err(SecurityError::DangerousHostPathMount(volume.source.clone()));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PrivilegeEscalationPolicy {
    Allowed,
    RequireApproval,
    Denied,
}

#[derive(Debug, Clone)]
pub enum DnsPolicy {
    ClusterFirst,
    Default,
    None,
}
```

### Network Security Implementation

```rust
// kina-cli/src/core/security/network.rs
pub struct NetworkSecurityManager {
    policies: Vec<NetworkSecurityPolicy>,
    firewall_rules: Vec<FirewallRule>,
}

#[derive(Debug, Clone)]
pub struct NetworkSecurityPolicy {
    pub name: String,
    pub namespace: String,
    pub pod_selector: HashMap<String, String>,
    pub ingress_rules: Vec<IngressRule>,
    pub egress_rules: Vec<EgressRule>,
}

#[derive(Debug, Clone)]
pub struct IngressRule {
    pub from: Vec<NetworkPeer>,
    pub ports: Vec<NetworkPort>,
}

#[derive(Debug, Clone)]
pub struct EgressRule {
    pub to: Vec<NetworkPeer>,
    pub ports: Vec<NetworkPort>,
}

#[derive(Debug, Clone)]
pub enum NetworkPeer {
    PodSelector(HashMap<String, String>),
    NamespaceSelector(HashMap<String, String>),
    IPBlock { cidr: String, except: Vec<String> },
}

impl NetworkSecurityManager {
    pub async fn apply_network_policies(
        &self,
        container_id: &str,
        policies: &[NetworkSecurityPolicy]
    ) -> Result<(), SecurityError> {
        for policy in policies {
            self.configure_container_network_policy(container_id, policy).await?;
        }
        Ok(())
    }

    async fn configure_container_network_policy(
        &self,
        container_id: &str,
        policy: &NetworkSecurityPolicy
    ) -> Result<(), SecurityError> {
        // Apple Container uses dedicated IPs, so we can configure
        // network policies at the VM level using iptables or pfctl

        let container_ip = self.get_container_ip(container_id).await?;

        // Configure ingress rules
        for rule in &policy.ingress_rules {
            self.apply_ingress_rule(container_ip, rule).await?;
        }

        // Configure egress rules
        for rule in &policy.egress_rules {
            self.apply_egress_rule(container_ip, rule).await?;
        }

        Ok(())
    }

    async fn apply_ingress_rule(
        &self,
        container_ip: std::net::IpAddr,
        rule: &IngressRule
    ) -> Result<(), SecurityError> {
        for peer in &rule.from {
            for port in &rule.ports {
                let firewall_rule = FirewallRule {
                    direction: Direction::Ingress,
                    source: self.resolve_network_peer(peer).await?,
                    destination: container_ip,
                    port: port.port,
                    protocol: port.protocol.clone(),
                    action: FirewallAction::Allow,
                };

                self.apply_firewall_rule(&firewall_rule).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub direction: Direction,
    pub source: std::net::IpAddr,
    pub destination: std::net::IpAddr,
    pub port: Option<u16>,
    pub protocol: Protocol,
    pub action: FirewallAction,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Ingress,
    Egress,
}

#[derive(Debug, Clone)]
pub enum FirewallAction {
    Allow,
    Deny,
    Log,
}
```

## Compliance and Audit Framework

### Security Audit Implementation

```rust
// kina-cli/src/core/security/audit.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditLog {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub user: String,
    pub container_id: Option<String>,
    pub cluster_name: Option<String>,
    pub details: HashMap<String, String>,
    pub compliance_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    ContainerCreated,
    ContainerStarted,
    ContainerStopped,
    PrivilegedAccess,
    NetworkPolicyViolation,
    VolumeMount,
    ImagePull,
    ClusterAccess,
    SecurityPolicyViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub struct SecurityAuditor {
    log_sink: Box<dyn AuditSink>,
    compliance_checker: ComplianceChecker,
}

#[async_trait::async_trait]
pub trait AuditSink: Send + Sync {
    async fn log_event(&self, event: &SecurityAuditLog) -> Result<(), AuditError>;
    async fn query_events(&self, query: &AuditQuery) -> Result<Vec<SecurityAuditLog>, AuditError>;
}

impl SecurityAuditor {
    pub async fn audit_container_creation(
        &self,
        container_id: &str,
        config: &ContainerConfig,
        user: &str
    ) -> Result<(), AuditError> {
        let event = SecurityAuditLog {
            timestamp: Utc::now(),
            event_type: AuditEventType::ContainerCreated,
            severity: if config.privileged { AuditSeverity::Warning } else { AuditSeverity::Info },
            user: user.to_string(),
            container_id: Some(container_id.to_string()),
            cluster_name: None,
            details: self.extract_container_details(config),
            compliance_tags: self.generate_compliance_tags(config),
        };

        self.log_sink.log_event(&event).await?;

        // Check for compliance violations
        self.compliance_checker.check_container_compliance(config, &event).await?;

        Ok(())
    }

    pub async fn audit_privileged_access(
        &self,
        container_id: &str,
        command: &[String],
        user: &str
    ) -> Result<(), AuditError> {
        let event = SecurityAuditLog {
            timestamp: Utc::now(),
            event_type: AuditEventType::PrivilegedAccess,
            severity: AuditSeverity::Warning,
            user: user.to_string(),
            container_id: Some(container_id.to_string()),
            cluster_name: None,
            details: {
                let mut details = HashMap::new();
                details.insert("command".to_string(), command.join(" "));
                details
            },
            compliance_tags: vec!["privileged-access".to_string()],
        };

        self.log_sink.log_event(&event).await?;
        Ok(())
    }

    fn extract_container_details(&self, config: &ContainerConfig) -> HashMap<String, String> {
        let mut details = HashMap::new();
        details.insert("image".to_string(), config.image.clone());
        details.insert("privileged".to_string(), config.privileged.to_string());

        if !config.volumes.is_empty() {
            let volume_info: Vec<String> = config.volumes.iter()
                .map(|v| format!("{}:{}", v.source, v.target))
                .collect();
            details.insert("volumes".to_string(), volume_info.join(","));
        }

        details
    }

    fn generate_compliance_tags(&self, config: &ContainerConfig) -> Vec<String> {
        let mut tags = Vec::new();

        if config.privileged {
            tags.push("privileged-container".to_string());
        }

        if config.volumes.iter().any(|v| !v.read_only) {
            tags.push("writable-volumes".to_string());
        }

        if config.resource_limits.is_none() {
            tags.push("unlimited-resources".to_string());
        }

        tags
    }
}

#[derive(Debug)]
pub struct ComplianceChecker {
    policies: Vec<CompliancePolicy>,
}

#[derive(Debug, Clone)]
pub struct CompliancePolicy {
    pub name: String,
    pub framework: ComplianceFramework,
    pub rules: Vec<ComplianceRule>,
}

#[derive(Debug, Clone)]
pub enum ComplianceFramework {
    SOC2,
    PCI,
    HIPAA,
    CIS,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ComplianceRule {
    pub id: String,
    pub description: String,
    pub severity: ComplianceSeverity,
    pub checker: Box<dyn ComplianceChecker>,
}

#[derive(Debug, Clone)]
pub enum ComplianceSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

## Security Best Practices

### Container Image Security

```rust
// kina-cli/src/core/security/image_security.rs
pub struct ImageSecurityScanner {
    vulnerability_db: VulnerabilityDatabase,
}

impl ImageSecurityScanner {
    pub async fn scan_image(&self, image: &str) -> Result<ImageSecurityReport, SecurityError> {
        // Pull image manifest
        let manifest = self.get_image_manifest(image).await?;

        // Scan for vulnerabilities
        let vulnerabilities = self.scan_vulnerabilities(&manifest).await?;

        // Check image signing
        let signature_valid = self.verify_image_signature(image).await?;

        // Analyze image configuration
        let config_issues = self.analyze_image_config(&manifest).await?;

        Ok(ImageSecurityReport {
            image: image.to_string(),
            vulnerabilities,
            signature_valid,
            config_issues,
            risk_score: self.calculate_risk_score(&vulnerabilities, &config_issues),
        })
    }

    async fn verify_image_signature(&self, image: &str) -> Result<bool, SecurityError> {
        // Implement image signature verification
        // Could integrate with Notary v2 or Cosign
        todo!("Implement image signature verification")
    }

    fn calculate_risk_score(&self, vulnerabilities: &[Vulnerability], config_issues: &[ConfigIssue]) -> RiskScore {
        let vuln_score = vulnerabilities.iter()
            .map(|v| v.severity.score())
            .sum::<f64>();

        let config_score = config_issues.iter()
            .map(|i| i.severity.score())
            .sum::<f64>();

        RiskScore::from_total(vuln_score + config_score)
    }
}

#[derive(Debug)]
pub struct ImageSecurityReport {
    pub image: String,
    pub vulnerabilities: Vec<Vulnerability>,
    pub signature_valid: bool,
    pub config_issues: Vec<ConfigIssue>,
    pub risk_score: RiskScore,
}

#[derive(Debug, Clone)]
pub struct Vulnerability {
    pub cve_id: String,
    pub severity: VulnerabilitySeverity,
    pub description: String,
    pub package: String,
    pub fixed_version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum VulnerabilitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl VulnerabilitySeverity {
    fn score(&self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 4.0,
            Self::High => 7.0,
            Self::Critical => 10.0,
        }
    }
}
```

### Runtime Security Monitoring

```rust
// kina-cli/src/core/security/runtime_monitor.rs
pub struct RuntimeSecurityMonitor {
    anomaly_detector: AnomalyDetector,
    threat_detector: ThreatDetector,
    alert_manager: AlertManager,
}

impl RuntimeSecurityMonitor {
    pub async fn monitor_container(&self, container_id: &str) -> Result<(), SecurityError> {
        // Monitor system calls
        let syscall_monitor = self.start_syscall_monitoring(container_id).await?;

        // Monitor network activity
        let network_monitor = self.start_network_monitoring(container_id).await?;

        // Monitor file system access
        let fs_monitor = self.start_filesystem_monitoring(container_id).await?;

        // Process monitoring results
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = syscall_monitor.next_event() => {
                        if let Some(suspicious) = self.analyze_syscall_event(event) {
                            self.alert_manager.send_alert(suspicious).await;
                        }
                    }
                    event = network_monitor.next_event() => {
                        if let Some(suspicious) = self.analyze_network_event(event) {
                            self.alert_manager.send_alert(suspicious).await;
                        }
                    }
                    event = fs_monitor.next_event() => {
                        if let Some(suspicious) = self.analyze_filesystem_event(event) {
                            self.alert_manager.send_alert(suspicious).await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn analyze_syscall_event(&self, event: SyscallEvent) -> Option<SecurityAlert> {
        // Detect suspicious system call patterns
        match event.syscall {
            Syscall::Ptrace => {
                Some(SecurityAlert::new(
                    AlertSeverity::High,
                    "Potential debugging/injection attempt".to_string(),
                    event.container_id,
                ))
            }
            Syscall::Mount if !event.allowed => {
                Some(SecurityAlert::new(
                    AlertSeverity::Medium,
                    "Unauthorized mount attempt".to_string(),
                    event.container_id,
                ))
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct SecurityAlert {
    pub severity: AlertSeverity,
    pub message: String,
    pub container_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

This security analysis provides comprehensive coverage of Apple Container's security model, implementation patterns, and compliance considerations for the kina CLI project.