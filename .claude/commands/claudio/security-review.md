---
description: "Comprehensive security review for Rust CLI applications with STRIDE methodology"
argument-hint: "[project_path] [review_scope] [output_format]"
---

Conduct comprehensive security reviews for Rust CLI applications using STRIDE methodology with specialized focus on container orchestration and Kubernetes integration security patterns.

**Security Review Capabilities:**
- **STRIDE Analysis**: Systematic threat modeling (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege)
- **Rust Security Patterns**: Memory safety, dependency auditing, secure coding practices
- **Container Security**: Image scanning, runtime security, privilege management
- **Kubernetes Security**: RBAC analysis, network policies, secret management
- **CLI Security**: Input validation, privilege handling, credential management

**Review Scope Options:**
- `full`: Complete security assessment across all threat vectors
- `container`: Container and orchestration security focus
- `api`: Kubernetes API security and RBAC analysis
- `dependencies`: Dependency vulnerability scanning and policy compliance
- `input`: Input validation and injection prevention analysis
- `privileges`: Privilege management and access control review
- `crypto`: Cryptographic usage and key management assessment

**Rust CLI Security Focus:**
This command specializes in security review for Rust CLI applications:

- **Memory Safety**: Leveraging Rust's safety guarantees, unsafe code review, dependency audit
- **Dependency Security**: cargo-audit integration, vulnerability scanning, license compliance
- **Container Integration**: Docker/Apple Container security, image scanning, runtime protection
- **Kubernetes Security**: API authentication, RBAC configuration, network security
- **CLI-Specific Threats**: Command injection, path traversal, privilege escalation

**Security Analysis Areas:**
- **Input Validation**: Command-line argument sanitization, configuration file parsing security
- **Authentication & Authorization**: API key management, OAuth integration, RBAC compliance
- **Data Protection**: Sensitive data handling, logging security, credential storage
- **Network Security**: TLS configuration, API communication security, certificate validation
- **Container Security**: Image vulnerabilities, runtime security, orchestration privileges

**STRIDE Methodology Application:**
- **Spoofing**: Identity verification, API authentication, certificate validation
- **Tampering**: Data integrity, configuration protection, binary signing
- **Repudiation**: Audit logging, action tracking, non-repudiation mechanisms
- **Information Disclosure**: Data classification, access controls, logging sanitization
- **Denial of Service**: Resource limits, rate limiting, graceful degradation
- **Elevation of Privilege**: Privilege separation, least privilege, capability restrictions

**Container & Kubernetes Security:**
- **Image Security**: Vulnerability scanning, minimal base images, supply chain security
- **Runtime Security**: Container isolation, syscall filtering, security contexts
- **Network Security**: Network policies, service mesh security, encrypted communication
- **RBAC Analysis**: Role-based access controls, service account security, permission auditing
- **Secret Management**: Kubernetes secrets, external secret management, rotation policies

**Example Usage:**
```bash
/claudio:security-review                                # Full security assessment
/claudio:security-review ./kina-cli container          # Container-focused review
/claudio:security-review . api --json                  # API security with JSON output
/claudio:security-review ../rust-project dependencies  # Dependency security audit
```

**Output Formats:**
- **Detailed Report**: Comprehensive security assessment with remediation recommendations
- **Executive Summary**: High-level security posture and priority recommendations
- **STRIDE Matrix**: Systematic threat analysis with risk ratings and mitigation strategies
- **Compliance Report**: Standards compliance (SOC2, PCI, GDPR) assessment

Task with subagent_type: "security-review-coordinator" - pass the project_path argument for comprehensive security review coordination using STRIDE methodology with specialized focus on Rust CLI applications, container security, and Kubernetes integration threat modeling.