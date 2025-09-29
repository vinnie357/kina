use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::apple_container::AppleContainerClient;
use super::kubernetes::KubernetesClient;
use super::types::{ClusterInfo, ClusterStatus, CreateClusterOptions, LoadImageOptions};
use crate::config::Config;

/// Cluster manager handles all cluster operations
pub struct ClusterManager {
    config: Config,
    apple_container: AppleContainerClient,
    kubernetes: KubernetesClient,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new(config: &Config) -> Result<Self> {
        let apple_container = AppleContainerClient::new(config)?;
        let kubernetes = KubernetesClient::new(config)?;

        Ok(Self {
            config: config.clone(),
            apple_container,
            kubernetes,
        })
    }

    /// Create a new Kubernetes cluster
    pub async fn create_cluster(&self, options: CreateClusterOptions) -> Result<()> {
        info!(
            "Creating cluster '{}' with image '{}'",
            options.name, options.image
        );

        // Check if cluster already exists
        if self.cluster_exists(&options.name).await? {
            return Err(anyhow::anyhow!("Cluster '{}' already exists", options.name));
        }

        // Create the cluster using Apple Container
        self.apple_container
            .create_cluster(&options)
            .await
            .context("Failed to create cluster using Apple Container")?;

        // Wait for cluster to be ready if requested
        if let Some(timeout) = options.wait_timeout {
            self.wait_for_cluster_ready(&options.name, timeout).await?;
        } else {
            // Even without explicit wait, give the cluster a moment to initialize
            // This ensures the API server is ready for CSR operations
            info!("Waiting briefly for cluster API server to be ready...");
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }

        // Auto-approve kubelet-serving CSRs after cluster creation (unless skipped)
        // This prevents TLS errors with kubectl logs/exec commands
        if options.skip_csr_approval {
            info!("Skipping kubelet CSR auto-approval (--skip-csr-approval specified)");
            warn!(
                "You may need to manually approve CSRs with: kina approve-csr {}",
                options.name
            );
        } else {
            info!("Bootstrapping kubelet CSR auto-approval...");
            if let Err(e) = self.bootstrap_kubelet_csrs(&options.name).await {
                warn!(
                    "Failed to bootstrap kubelet CSRs (this may cause TLS errors): {}",
                    e
                );
                warn!(
                    "You can manually approve CSRs later with: kina approve-csr {}",
                    options.name
                );
            }
        }

        info!("Cluster '{}' created successfully", options.name);
        Ok(())
    }

    /// Delete a Kubernetes cluster
    pub async fn delete_cluster(&self, name: &str) -> Result<()> {
        info!("Deleting cluster '{}'", name);

        if !self.cluster_exists(name).await? {
            warn!("Cluster '{}' does not exist", name);
            return Ok(());
        }

        self.apple_container
            .delete_cluster(name)
            .await
            .context("Failed to delete cluster")?;

        // Clean up kubeconfig
        self.cleanup_kubeconfig(name).await?;

        info!("Cluster '{}' deleted successfully", name);
        Ok(())
    }

    /// Delete all clusters
    pub async fn delete_all_clusters(&self) -> Result<()> {
        let clusters = self.list_clusters().await?;

        if clusters.is_empty() {
            info!("No clusters to delete");
            return Ok(());
        }

        for cluster in clusters {
            if let Err(e) = self.delete_cluster(&cluster.name).await {
                warn!("Failed to delete cluster '{}': {}", cluster.name, e);
            }
        }

        Ok(())
    }

    /// List all existing clusters
    pub async fn list_clusters(&self) -> Result<Vec<ClusterInfo>> {
        debug!("Listing clusters");

        self.apple_container
            .list_clusters()
            .await
            .context("Failed to list clusters")
    }

    /// Check if a cluster exists
    pub async fn cluster_exists(&self, name: &str) -> Result<bool> {
        let clusters = self.list_clusters().await?;
        Ok(clusters.iter().any(|c| c.name == name))
    }

    /// Get kubeconfig for a cluster
    pub async fn get_kubeconfig(&self, name: &str) -> Result<String> {
        debug!("Getting kubeconfig for cluster '{}'", name);

        if !self.cluster_exists(name).await? {
            return Err(anyhow::anyhow!("Cluster '{}' does not exist", name));
        }

        self.apple_container
            .get_kubeconfig(name)
            .await
            .context("Failed to get kubeconfig")
    }

    /// Get nodes in a cluster
    pub async fn get_nodes(&self, name: &str) -> Result<Vec<String>> {
        debug!("Getting nodes for cluster '{}'", name);

        if !self.cluster_exists(name).await? {
            return Err(anyhow::anyhow!("Cluster '{}' does not exist", name));
        }

        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' not found", name))?;

        Ok(cluster.nodes.iter().map(|n| n.name.clone()).collect())
    }

    /// Load a container image into a cluster
    pub async fn load_image(&self, options: LoadImageOptions) -> Result<()> {
        info!(
            "Loading image '{}' into cluster '{}'",
            options.image, options.cluster
        );

        if !self.cluster_exists(&options.cluster).await? {
            return Err(anyhow::anyhow!(
                "Cluster '{}' does not exist",
                options.cluster
            ));
        }

        self.apple_container
            .load_image(&options)
            .await
            .context("Failed to load image into cluster")?;

        info!(
            "Image '{}' loaded successfully into cluster '{}'",
            options.image, options.cluster
        );
        Ok(())
    }

    /// Wait for a cluster to be ready
    async fn wait_for_cluster_ready(&self, name: &str, timeout_seconds: u64) -> Result<()> {
        info!(
            "Waiting for cluster '{}' to be ready (timeout: {}s)",
            name, timeout_seconds
        );

        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for cluster '{}' to be ready",
                    name
                ));
            }

            let clusters = self.list_clusters().await?;
            if let Some(cluster) = clusters.iter().find(|c| c.name == name) {
                match cluster.status {
                    ClusterStatus::Running => {
                        info!("Cluster '{}' is ready", name);
                        return Ok(());
                    }
                    ClusterStatus::Error => {
                        return Err(anyhow::anyhow!("Cluster '{}' failed to start", name));
                    }
                    _ => {
                        debug!("Cluster '{}' status: {:?}", name, cluster.status);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Cluster '{}' not found", name));
            }
        }
    }

    /// Clean up kubeconfig for a deleted cluster
    async fn cleanup_kubeconfig(&self, name: &str) -> Result<()> {
        debug!("Cleaning up kubeconfig for cluster '{}'", name);

        let kubeconfig_path = self
            .config
            .kubernetes
            .kubeconfig_dir
            .join(format!("{}.yaml", name));

        if kubeconfig_path.exists() {
            std::fs::remove_file(&kubeconfig_path).context("Failed to remove kubeconfig file")?;
            debug!("Removed kubeconfig file: {}", kubeconfig_path.display());
        }

        Ok(())
    }

    /// Get detailed status for a specific cluster
    pub async fn get_cluster_status(&self, name: &str) -> Result<ClusterInfo> {
        debug!("Getting detailed status for cluster '{}'", name);

        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' does not exist", name))?;

        Ok(cluster)
    }

    /// Bootstrap kubelet CSR auto-approval for a cluster
    /// This prevents TLS errors with kubectl logs/exec by auto-approving kubelet-serving CSRs
    async fn bootstrap_kubelet_csrs(&self, cluster_name: &str) -> Result<()> {
        info!(
            "Bootstrapping kubelet CSR auto-approval for cluster '{}'",
            cluster_name
        );

        // Get kubeconfig path where Apple Container actually saves it (~/.kube/{cluster_name})
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = std::path::Path::new(&home_dir).join(".kube");
        let kubeconfig_path = kube_dir.join(cluster_name);

        let kubeconfig_str = kubeconfig_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid kubeconfig path"))?;

        info!(
            "🔧 Using kubeconfig path for CSR auto-approval: {}",
            kubeconfig_str
        );

        // Wait for and auto-approve kubelet-serving CSRs for 60 seconds
        // This should be enough time for kubelet to generate its serving CSRs
        self.kubernetes
            .bootstrap_approve_kubelet_csrs(kubeconfig_str, 60)
            .await
            .context("Failed to bootstrap kubelet CSR auto-approval")?;

        info!(
            "Kubelet CSR bootstrap completed for cluster '{}'",
            cluster_name
        );
        Ok(())
    }

    /// Manually approve any pending kubelet-serving CSRs for a cluster
    /// This can be used to fix TLS issues in existing clusters
    pub async fn approve_kubelet_csrs(&self, cluster_name: &str) -> Result<()> {
        info!(
            "Approving pending kubelet CSRs for cluster '{}'",
            cluster_name
        );

        if !self.cluster_exists(cluster_name).await? {
            return Err(anyhow::anyhow!("Cluster '{}' does not exist", cluster_name));
        }

        // Get kubeconfig path for the cluster
        let kubeconfig_path = self
            .config
            .kubernetes
            .kubeconfig_dir
            .join(format!("{}.yaml", cluster_name));

        let kubeconfig_str = kubeconfig_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid kubeconfig path"))?;

        // Auto-approve all pending kubelet-serving CSRs
        self.kubernetes
            .auto_approve_kubelet_csrs(kubeconfig_str)
            .await
            .context("Failed to approve kubelet CSRs")?;

        info!(
            "Kubelet CSR approval completed for cluster '{}'",
            cluster_name
        );
        Ok(())
    }
}
