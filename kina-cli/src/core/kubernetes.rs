#![allow(dead_code)]
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::config::Config;

/// Client for Kubernetes operations
#[allow(dead_code)]
pub struct KubernetesClient {
    config: Config,
    kubectl_path: String,
}

impl KubernetesClient {
    /// Create a new Kubernetes client
    pub fn new(config: &Config) -> Result<Self> {
        let kubectl_path = if let Some(path) = &config.kubernetes.kubectl_path {
            path.to_string_lossy().to_string()
        } else {
            Self::detect_kubectl_path()?
        };

        Ok(Self {
            config: config.clone(),
            kubectl_path,
        })
    }

    /// Detect kubectl CLI path
    fn detect_kubectl_path() -> Result<String> {
        if let Ok(output) = std::process::Command::new("which").arg("kubectl").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    info!("Found kubectl at: {}", path);
                    return Ok(path);
                }
            }
        }

        Err(anyhow::anyhow!("kubectl not found in PATH"))
    }

    /// Check if cluster is accessible via kubectl
    pub async fn check_cluster_access(&self, kubeconfig_path: &str) -> Result<bool> {
        debug!(
            "Checking cluster access with kubeconfig: {}",
            kubeconfig_path
        );

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("cluster-info");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute kubectl cluster-info")?;

        Ok(output.status.success())
    }

    /// Get cluster nodes
    pub async fn get_nodes(&self, kubeconfig_path: &str) -> Result<Vec<String>> {
        debug!("Getting nodes with kubeconfig: {}", kubeconfig_path);

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("get")
            .arg("nodes")
            .arg("-o")
            .arg("name");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute kubectl get nodes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("kubectl get nodes failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let nodes: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_start_matches("node/").to_string())
            .collect();

        Ok(nodes)
    }

    /// Wait for cluster to be ready
    pub async fn wait_for_cluster_ready(
        &self,
        kubeconfig_path: &str,
        timeout_seconds: u64,
    ) -> Result<()> {
        info!(
            "Waiting for cluster to be ready (timeout: {}s)",
            timeout_seconds
        );

        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        while start_time.elapsed() < timeout {
            if self.check_cluster_access(kubeconfig_path).await? {
                // Check if nodes are ready
                if let Ok(nodes) = self.get_nodes(kubeconfig_path).await {
                    if !nodes.is_empty() && self.check_nodes_ready(kubeconfig_path).await? {
                        info!("Cluster is ready with {} nodes", nodes.len());
                        return Ok(());
                    }
                }
            }

            debug!("Cluster not ready yet, waiting...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        Err(anyhow::anyhow!("Timeout waiting for cluster to be ready"))
    }

    /// Check if all nodes are ready
    pub async fn check_nodes_ready(&self, kubeconfig_path: &str) -> Result<bool> {
        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("get")
            .arg("nodes")
            .arg("--no-headers")
            .arg("-o")
            .arg("custom-columns=STATUS:.status.conditions[-1].type");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check node readiness")?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let all_ready = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .all(|line| line.trim() == "Ready");

        Ok(all_ready)
    }

    /// Apply a Kubernetes manifest
    pub async fn apply_manifest(&self, kubeconfig_path: &str, manifest: &str) -> Result<()> {
        debug!("Applying Kubernetes manifest");

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("apply")
            .arg("-f")
            .arg("-");

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .context("Failed to spawn kubectl apply process")?;

        // Write manifest to stdin
        if let Some(stdin) = child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(manifest.as_bytes())
                .await
                .context("Failed to write manifest to kubectl stdin")?;
        }

        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait for kubectl apply to complete")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("kubectl apply failed: {}", stderr));
        }

        Ok(())
    }

    /// Delete resources by label selector
    pub async fn delete_by_selector(
        &self,
        kubeconfig_path: &str,
        resource_type: &str,
        selector: &str,
    ) -> Result<()> {
        debug!("Deleting {} with selector: {}", resource_type, selector);

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("delete")
            .arg(resource_type)
            .arg("-l")
            .arg(selector);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute kubectl delete")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("kubectl delete warning: {}", stderr);
        }

        Ok(())
    }

    /// List all Certificate Signing Requests with their status
    pub async fn list_all_csrs(&self, kubeconfig_path: &str) -> Result<Vec<(String, String)>> {
        debug!("Listing all CSRs with status information");

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("get")
            .arg("csr")
            .arg("-o")
            .arg("custom-columns=NAME:.metadata.name,SIGNER:.spec.signerName,STATUS:.status.conditions[0].type")
            .arg("--no-headers");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list all CSRs")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to list CSRs: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let csrs: Vec<(String, String)> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    (format!("{} ({})", parts[0], parts[1]), parts[2].to_string())
                } else {
                    (line.to_string(), "Unknown".to_string())
                }
            })
            .collect();

        Ok(csrs)
    }

    /// Get pending kubelet-serving Certificate Signing Requests
    pub async fn get_pending_kubelet_csrs(&self, kubeconfig_path: &str) -> Result<Vec<String>> {
        debug!("Getting pending kubelet-serving CSRs");

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("get")
            .arg("csr")
            .arg("-o")
            .arg("jsonpath={range .items[?(@.spec.signerName==\"kubernetes.io/kubelet-serving\")]}{.metadata.name}{\"\\n\"}{end}");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get CSRs")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to get CSRs: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let csrs: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();

        Ok(csrs)
    }

    /// Approve a Certificate Signing Request
    pub async fn approve_csr(&self, kubeconfig_path: &str, csr_name: &str) -> Result<()> {
        debug!("Approving CSR: {}", csr_name);

        let mut cmd = Command::new(&self.kubectl_path);
        cmd.arg("--kubeconfig")
            .arg(kubeconfig_path)
            .arg("certificate")
            .arg("approve")
            .arg(csr_name);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to approve CSR")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to approve CSR {}: {}",
                csr_name,
                stderr
            ));
        }

        info!("Approved CSR: {}", csr_name);
        Ok(())
    }

    /// Auto-approve all pending kubelet-serving CSRs
    pub async fn auto_approve_kubelet_csrs(&self, kubeconfig_path: &str) -> Result<()> {
        info!("Auto-approving pending kubelet-serving CSRs");

        let pending_csrs = self.get_pending_kubelet_csrs(kubeconfig_path).await?;

        if pending_csrs.is_empty() {
            debug!("No pending kubelet-serving CSRs found");
            return Ok(());
        }

        info!("Found {} pending kubelet-serving CSRs", pending_csrs.len());

        for csr_name in pending_csrs {
            if let Err(e) = self.approve_csr(kubeconfig_path, &csr_name).await {
                warn!("Failed to approve CSR {}: {}", csr_name, e);
            }
        }

        info!("Completed auto-approval of kubelet-serving CSRs");
        Ok(())
    }

    /// Wait for kubelet-serving CSRs to appear and auto-approve them
    /// This is needed during cluster bootstrap when kubelet TLS bootstrap is enabled
    pub async fn bootstrap_approve_kubelet_csrs(
        &self,
        kubeconfig_path: &str,
        timeout_seconds: u64,
    ) -> Result<()> {
        info!("🔐 Starting enhanced kubelet CSR bootstrap auto-approval");
        info!("⏱️  Timeout: {}s", timeout_seconds);
        info!("📋 Kubeconfig: {}", kubeconfig_path);

        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);
        let mut approved_count = 0;
        let mut iteration = 0;

        // First, list all existing CSRs to understand the current state
        match self.list_all_csrs(kubeconfig_path).await {
            Ok(all_csrs) => {
                info!("📊 Initial CSR state: {} total CSRs found", all_csrs.len());
                for csr in &all_csrs {
                    info!("   CSR: {} - Status: {}", csr.0, csr.1);
                }
            }
            Err(e) => {
                warn!("⚠️  Could not list initial CSR state: {}", e);
            }
        }

        while start_time.elapsed() < timeout {
            iteration += 1;
            let elapsed = start_time.elapsed();

            debug!(
                "🔄 CSR check iteration {} (elapsed: {}s)",
                iteration,
                elapsed.as_secs()
            );

            match self.get_pending_kubelet_csrs(kubeconfig_path).await {
                Ok(pending_csrs) => {
                    if !pending_csrs.is_empty() {
                        info!(
                            "✅ Found {} pending kubelet-serving CSRs",
                            pending_csrs.len()
                        );

                        for csr_name in &pending_csrs {
                            info!("   🔍 Processing CSR: {}", csr_name);
                        }

                        for csr_name in pending_csrs {
                            match self.approve_csr(kubeconfig_path, &csr_name).await {
                                Ok(()) => {
                                    approved_count += 1;
                                    info!("✅ Successfully approved CSR: {}", csr_name);
                                }
                                Err(e) => {
                                    warn!("❌ Failed to approve CSR {}: {}", csr_name, e);
                                }
                            }
                        }
                    } else {
                        debug!(
                            "🔍 No pending kubelet-serving CSRs found (iteration {})",
                            iteration
                        );
                    }
                }
                Err(e) => {
                    warn!("⚠️  Error checking for pending CSRs: {}", e);
                }
            }

            // Log progress every 10 seconds
            if elapsed.as_secs() % 10 == 0 && elapsed.as_secs() > 0 {
                info!(
                    "⏳ CSR approval progress: {}s / {}s (approved: {})",
                    elapsed.as_secs(),
                    timeout_seconds,
                    approved_count
                );
            }

            // Wait a bit before checking again
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        info!(
            "🏁 CSR bootstrap completed after {}s",
            start_time.elapsed().as_secs()
        );

        if approved_count > 0 {
            info!(
                "✅ Bootstrap CSR approval completed: {} CSRs approved",
                approved_count
            );
        } else {
            warn!("❌ No kubelet-serving CSRs were found during bootstrap period");
            // List all CSRs again to see what's available
            if let Ok(all_csrs) = self.list_all_csrs(kubeconfig_path).await {
                warn!("📊 Final CSR state: {} total CSRs", all_csrs.len());
                for csr in &all_csrs {
                    warn!("   CSR: {} - Status: {}", csr.0, csr.1);
                }
            }
        }

        Ok(())
    }
}
