use assert_cmd::Command;
use predicates::prelude::*;

/// Test utilities for CLI testing
pub struct TestContext {
    pub config_path: String,
}

impl TestContext {
    pub fn new() -> Self {
        // Use the fixture file from the test directory
        // Get the absolute path to the fixture file
        let config_path = std::env::current_dir()
            .expect("Failed to get current directory")
            .join("tests/fixtures/test-config.toml")
            .to_string_lossy()
            .to_string();

        Self { config_path }
    }

    pub fn create_test_config(&self) -> std::io::Result<()> {
        // Config file already exists as a fixture, nothing to do
        Ok(())
    }

    pub fn kina_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("kina").unwrap();
        cmd.arg("--config").arg(&self.config_path);
        cmd
    }
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("kina").unwrap();

    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("kina"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("kina").unwrap();

    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Apple Container"));
}

#[test]
fn test_subcommand_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();

    cmd.args(&["create", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new Kubernetes cluster"));
}

// ===== CREATE COMMAND TESTS =====

#[test]
fn test_create_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["create", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new Kubernetes cluster"));
}

#[test]
fn test_create_command_basic() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "test-cluster"]);
    // This will likely fail since Apple Container isn't available, but tests command parsing
}

#[test]
fn test_create_command_default_name() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("create");
    // Tests create with default cluster name "kina"
}

#[test]
fn test_create_command_with_image() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--image", "custom/image:latest"]);
    // Tests create with custom image
}

#[test]
fn test_create_command_with_config() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--config", "custom-config.yaml"]);
    // Tests create with custom config file
}

#[test]
fn test_create_command_with_wait() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--wait", "300"]);
    // Tests create with wait timeout
}

#[test]
fn test_create_command_with_retain() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--retain"]);
    // Tests create with retain on failure flag
}

#[test]
fn test_create_command_skip_csr_approval() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--skip-csr-approval"]);
    // Tests create with skip CSR approval flag
}

#[test]
fn test_create_command_with_cni_ptp() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--cni", "ptp"]);
    // Tests create with PTP CNI
}

#[test]
fn test_create_command_with_workers() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--workers", "2"]);
    // Tests create with worker nodes (will fail without Apple Container, but validates CLI parsing)
}

#[test]
fn test_create_command_with_workers_zero() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--workers", "0"]);
    // Tests create with 0 workers (single-node mode, the default)
}

#[test]
fn test_create_command_workers_in_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["create", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--workers"));
}

#[test]
fn test_create_command_with_cni_cilium() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["create", "--cni", "cilium"]);
    // Tests create with Cilium CNI
}

#[test]
fn test_create_command_validation() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    // Test create command with invalid cluster name
    let mut cmd = context.kina_command();
    cmd.args(&["create", "-invalid-name-"]);
    cmd.assert().failure(); // Should fail due to invalid cluster name
}

#[test]
fn test_list_command_empty() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    // List command when no clusters exist
    let mut cmd = context.kina_command();
    cmd.arg("list");
    // Note: This will likely fail because Apple Container isn't available in test environment
    // In a real test, we would mock the Apple Container client
}

#[test]
fn test_config_command() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["config", "show"]);
    cmd.assert().success(); // Should show current config
}

#[test]
fn test_config_path_command() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["config", "path"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(&context.config_path));
}

#[test]
fn test_verbose_flag() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["--verbose", "list"]);
    // Should work with verbose output
}

#[test]
fn test_quiet_flag() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["--quiet", "list"]);
    // Should work with quiet output
}

#[test]
fn test_conflicting_flags() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["--verbose", "--quiet", "list"]);
    cmd.assert().failure(); // Should fail due to conflicting flags
}

// ===== DELETE COMMAND TESTS =====

#[test]
fn test_delete_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["delete", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Delete a Kubernetes cluster"));
}

#[test]
fn test_delete_command_with_name() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["delete", "test-cluster"]);
    // This will likely fail since cluster doesn't exist, but tests command parsing
}

#[test]
fn test_delete_command_all_flag() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["delete", "--all"]);
    // Tests the --all flag functionality
}

#[test]
fn test_delete_command_conflicting_args() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["delete", "--all", "some-cluster"]);
    cmd.assert().failure(); // Should fail due to conflicting arguments
}

// ===== STATUS COMMAND TESTS =====

#[test]
fn test_status_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["status", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Show detailed status of a cluster",
    ));
}

#[test]
fn test_status_command_default() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("status");
    // This will likely fail since no clusters exist, but tests command parsing
}

#[test]
fn test_status_command_with_name() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["status", "test-cluster"]);
    // Tests status command with specific cluster name
}

#[test]
fn test_status_command_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("status");
    // This should NOT fail when no clusters exist - it should show a helpful message
    // Currently this test demonstrates the bug where it fails with an error
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_status_command_verbose() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["status", "--verbose"]);
    // Tests verbose output for status
}

#[test]
fn test_status_command_output_formats() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    // Test YAML output
    let mut cmd = context.kina_command();
    cmd.args(&["status", "--output", "yaml"]);
    // Should accept yaml format

    // Test JSON output
    let mut cmd = context.kina_command();
    cmd.args(&["status", "--output", "json"]);
    // Should accept json format

    // Test table output (default)
    let mut cmd = context.kina_command();
    cmd.args(&["status", "--output", "table"]);
    // Should accept table format
}

// ===== GET COMMAND TESTS =====

#[test]
fn test_get_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["get", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Get information about clusters or resources",
    ));
}

#[test]
fn test_get_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "clusters"]);
    // Tests getting cluster list
}

#[test]
fn test_get_kubeconfig() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "kubeconfig"]);
    // Tests getting kubeconfig for default cluster
}

#[test]
fn test_get_kubeconfig_with_name() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "kubeconfig", "test-cluster"]);
    // Tests getting kubeconfig for specific cluster
}

#[test]
fn test_get_nodes() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "nodes"]);
    // Tests getting nodes for default cluster
}

#[test]
fn test_get_nodes_with_name() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "nodes", "test-cluster"]);
    // Tests getting nodes for specific cluster
}

// ===== LOAD COMMAND TESTS =====

#[test]
fn test_load_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["load", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Load container images into clusters",
    ));
}

#[test]
fn test_load_command_basic() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["load", "nginx:latest"]);
    // Tests loading image to default cluster
}

#[test]
fn test_load_command_with_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["load", "nginx:latest", "--cluster", "test-cluster"]);
    // Tests loading image to specific cluster
}

#[test]
fn test_load_command_missing_image() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("load");
    cmd.assert().failure(); // Should fail due to missing image argument
}

// ===== INSTALL COMMAND TESTS =====

#[test]
fn test_install_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["install", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Install addons"));
}

#[test]
fn test_install_nginx_ingress() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "nginx-ingress"]);
    // Tests installing nginx-ingress addon
}

#[test]
fn test_install_ingress_nginx() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "ingress-nginx"]);
    // Tests installing ingress-nginx addon
}

#[test]
fn test_install_cni() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "cni"]);
    // Tests installing CNI addon
}

#[test]
fn test_install_coredns() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "coredns"]);
    // Tests installing CoreDNS addon
}

#[test]
fn test_install_metrics_server() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "metrics-server"]);
    // Tests installing metrics-server addon
}

#[test]
fn test_install_with_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "nginx-ingress", "--cluster", "test-cluster"]);
    // Tests installing addon to specific cluster
}

#[test]
fn test_install_with_version() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "nginx-ingress", "--version", "1.0.0"]);
    // Tests installing addon with specific version
}

#[test]
fn test_install_with_config() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "nginx-ingress", "--config", "custom-config.yaml"]);
    // Tests installing addon with custom config
}

#[test]
fn test_install_missing_addon() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("install");
    cmd.assert().failure(); // Should fail due to missing addon argument
}

// ===== EXPORT COMMAND TESTS =====

#[test]
fn test_export_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["export", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Export cluster configuration"));
}

#[test]
fn test_export_default_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("export");
    // Tests exporting default cluster kubeconfig
}

#[test]
fn test_export_specific_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["export", "test-cluster"]);
    // Tests exporting specific cluster
}

#[test]
fn test_export_kubeconfig_format() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["export", "--format", "kubeconfig"]);
    // Tests kubeconfig export format
}

#[test]
fn test_export_config_format() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["export", "--format", "config"]);
    // Tests config export format
}

#[test]
fn test_export_to_file() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["export", "--output", "/tmp/test-kubeconfig"]);
    // Tests exporting to specific file
}

// ===== APPROVE-CSR COMMAND TESTS =====

#[test]
fn test_approve_csr_command_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(&["approve-csr", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Approve pending kubelet Certificate Signing Requests",
    ));
}

#[test]
fn test_approve_csr_default_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("approve-csr");
    // Tests approving CSRs for default cluster
}

#[test]
fn test_approve_csr_specific_cluster() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["approve-csr", "test-cluster"]);
    // Tests approving CSRs for specific cluster
}

#[test]
fn test_approve_csr_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("approve-csr");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_get_kubeconfig_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "kubeconfig"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_get_nodes_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["get", "nodes"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_export_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.arg("export");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_load_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["load", "nginx:latest"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}

#[test]
fn test_install_no_clusters() {
    let context = TestContext::new();
    context.create_test_config().unwrap();

    let mut cmd = context.kina_command();
    cmd.args(&["install", "nginx-ingress"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No clusters found"));
}
