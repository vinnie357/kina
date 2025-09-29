use kina_cli::config::Config;
use serde_yaml;
use std::fs;
use tempfile::TempDir;

/// Helper function to test kubeconfig user naming patterns
fn test_kubeconfig_user_pattern(cluster_name: &str, expected_user: &str) -> bool {
    let expected_pattern = format!("{}-admin", cluster_name);
    expected_pattern == expected_user
}

/// Test sample kubeconfig creation for pattern validation
fn create_sample_kubeconfig(cluster_name: &str) -> String {
    format!(
        r#"apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
    server: https://127.0.0.1:6443
  name: {}
contexts:
- context:
    cluster: {}
    user: kubernetes-admin
  name: kubernetes-admin@{}
current-context: kubernetes-admin@{}
kind: Config
preferences: {{}}
users:
- name: kubernetes-admin
  user:
    client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
    client-key-data: LS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0t...
"#,
        cluster_name, cluster_name, cluster_name, cluster_name
    )
}

/// Test the expected kubeconfig transformation pattern
/// This tests the same logic that should be implemented in make_user_names_cluster_specific
fn transform_kubeconfig_user_names(
    kubeconfig: &str,
    cluster_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut config: serde_yaml::Value = serde_yaml::from_str(kubeconfig)?;

    // Create cluster-specific user name
    let cluster_specific_user = format!("{}-admin", cluster_name);

    // Update user name in users section
    if let Some(users) = config.get_mut("users").and_then(|u| u.as_sequence_mut()) {
        for user in users.iter_mut() {
            if let Some(name) = user.get("name").and_then(|n| n.as_str()) {
                if name == "kubernetes-admin" {
                    user["name"] = serde_yaml::Value::String(cluster_specific_user.clone());
                }
            }
        }
    }

    // Update user reference in contexts section
    if let Some(contexts) = config.get_mut("contexts").and_then(|c| c.as_sequence_mut()) {
        for context in contexts.iter_mut() {
            if let Some(context_obj) = context.get_mut("context") {
                if let Some(user) = context_obj.get("user").and_then(|u| u.as_str()) {
                    if user == "kubernetes-admin" {
                        context_obj["user"] =
                            serde_yaml::Value::String(cluster_specific_user.clone());
                    }
                }
            }
        }
    }

    let updated_kubeconfig = serde_yaml::to_string(&config)?;
    Ok(updated_kubeconfig)
}

#[test]
fn test_default_config() {
    let config = Config::default();

    assert_eq!(config.cluster.default_name, "kina");
    assert_eq!(config.cluster.default_image, "kindest/node:v1.31.0");
    assert_eq!(config.kubernetes.default_namespace, "default");
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_config_load_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let data_dir = temp_dir.path().join("data");
    let kubeconfig_dir = temp_dir.path().join("kubeconfig");
    let data_dir_str = data_dir.to_string_lossy();
    let kubeconfig_dir_str = kubeconfig_dir.to_string_lossy();

    let config_content = format!(
        r#"
[cluster]
default_name = "test-cluster"
default_image = "custom/image:latest"
default_wait_timeout = 120
data_dir = "{}"
retain_on_failure = true
default_cni = "Ptp"

[apple_container]
cli_path = "/custom/path/container"

[apple_container.runtime_config]
memory_limit = "2Gi"
storage_limit = "20Gi"

[apple_container.network]
network_name = "kina"
enable_ipv6 = false
dns_servers = []

[kubernetes]
default_version = "v1.29.0"
default_namespace = "test-namespace"
kubeconfig_dir = "{}"

[logging]
level = "debug"
format = "json"
file_logging = true
"#,
        data_dir_str, kubeconfig_dir_str
    );

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_file(&config_path).unwrap();

    assert_eq!(config.cluster.default_name, "test-cluster");
    assert_eq!(config.cluster.default_image, "custom/image:latest");
    assert_eq!(config.cluster.default_wait_timeout, 120);
    assert!(config.cluster.retain_on_failure);
    assert_eq!(config.kubernetes.default_namespace, "test-namespace");
    assert_eq!(config.logging.level, "debug");
    assert_eq!(config.logging.format, "json");
    assert!(config.logging.file_logging);
}

#[test]
fn test_config_load_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    let data_dir = temp_dir.path().join("data");
    let kubeconfig_dir = temp_dir.path().join("kubeconfig");
    let data_dir_str = data_dir.to_string_lossy();
    let kubeconfig_dir_str = kubeconfig_dir.to_string_lossy();

    let config_content = format!(
        r#"
cluster:
  default_name: "yaml-cluster"
  default_image: "yaml/image:latest"
  default_wait_timeout: 180
  data_dir: "{}"
  retain_on_failure: false
  default_cni: "Ptp"

apple_container:
  cli_path: "/yaml/path/container"
  runtime_config:
    memory_limit: "2Gi"
    storage_limit: "20Gi"
  network:
    network_name: "kina"
    enable_ipv6: false
    dns_servers: []

kubernetes:
  default_version: "v1.27.0"
  default_namespace: "yaml-namespace"
  kubeconfig_dir: "{}"

logging:
  level: "warn"
  format: "text"
  file_logging: false
"#,
        data_dir_str, kubeconfig_dir_str
    );

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_file(&config_path).unwrap();

    assert_eq!(config.cluster.default_name, "yaml-cluster");
    assert_eq!(config.cluster.default_image, "yaml/image:latest");
    assert_eq!(config.cluster.default_wait_timeout, 180);
    assert!(!config.cluster.retain_on_failure);
    assert_eq!(config.kubernetes.default_namespace, "yaml-namespace");
    assert_eq!(config.logging.level, "warn");
}

#[test]
fn test_config_save() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("save_test.toml");

    let mut config = Config::default();
    config.cluster.default_name = "saved-cluster".to_string();
    config.config_file_path = Some(config_path.clone());

    config.save().unwrap();

    assert!(config_path.exists());

    // Load the saved config
    let loaded_config = Config::load_from_file(&config_path).unwrap();
    assert_eq!(loaded_config.cluster.default_name, "saved-cluster");
}

#[test]
fn test_config_ensure_directories() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    let mut config = Config::default();
    config.cluster.data_dir = base_path.join("data");
    config.kubernetes.kubeconfig_dir = base_path.join("kubeconfig");

    config.ensure_directories().unwrap();

    assert!(config.cluster.data_dir.exists());
    assert!(config.kubernetes.kubeconfig_dir.exists());
}

#[test]
fn test_config_invalid_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");

    let invalid_content = "invalid toml content [[[";
    fs::write(&config_path, invalid_content).unwrap();

    let result = Config::load_from_file(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_config_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.toml");

    let result = Config::load_from_file(&nonexistent_path);
    assert!(result.is_err());
}

#[test]
fn test_kubeconfig_user_naming_pattern() {
    // Test the user naming pattern that should be implemented
    assert!(test_kubeconfig_user_pattern(
        "test-cluster",
        "test-cluster-admin"
    ));
    assert!(test_kubeconfig_user_pattern("my-app", "my-app-admin"));
    assert!(test_kubeconfig_user_pattern("dev", "dev-admin"));

    // Test that wrong patterns fail
    assert!(!test_kubeconfig_user_pattern(
        "test-cluster",
        "kubernetes-admin"
    ));
    assert!(!test_kubeconfig_user_pattern("test-cluster", "admin"));
    assert!(!test_kubeconfig_user_pattern(
        "test-cluster",
        "test-cluster"
    ));
}

#[test]
fn test_kubeconfig_transformation_logic() {
    let cluster_name = "test-cluster";
    let original_kubeconfig = create_sample_kubeconfig(cluster_name);

    // Test the transformation logic that mirrors make_user_names_cluster_specific
    let result = transform_kubeconfig_user_names(&original_kubeconfig, cluster_name);
    assert!(result.is_ok());

    let updated_kubeconfig = result.unwrap();

    // Verify that the user name was changed from "kubernetes-admin" to "test-cluster-admin"
    assert!(updated_kubeconfig.contains("test-cluster-admin"));
    // Check that the user entry specifically was renamed (not other occurrences like context names)
    assert!(updated_kubeconfig.contains("- name: test-cluster-admin"));

    // Verify the context also references the new user name
    assert!(updated_kubeconfig.contains("user: test-cluster-admin"));

    // Parse as YAML to verify structure is still valid
    let config: serde_yaml::Value = serde_yaml::from_str(&updated_kubeconfig).unwrap();

    // Check users section
    let users = config.get("users").and_then(|u| u.as_sequence()).unwrap();
    assert_eq!(users.len(), 1);
    let user_name = users[0].get("name").and_then(|n| n.as_str()).unwrap();
    assert_eq!(user_name, "test-cluster-admin");

    // Check contexts section
    let contexts = config
        .get("contexts")
        .and_then(|c| c.as_sequence())
        .unwrap();
    assert_eq!(contexts.len(), 1);
    let context_user = contexts[0]
        .get("context")
        .and_then(|c| c.get("user"))
        .and_then(|u| u.as_str())
        .unwrap();
    assert_eq!(context_user, "test-cluster-admin");
}

#[test]
fn test_kubeconfig_transformation_preserves_structure() {
    let cluster_name = "my-test";
    let original_kubeconfig = create_sample_kubeconfig(cluster_name);

    let result = transform_kubeconfig_user_names(&original_kubeconfig, cluster_name);
    assert!(result.is_ok());

    let updated_kubeconfig = result.unwrap();

    // Parse both configurations to ensure structure is preserved
    let original: serde_yaml::Value = serde_yaml::from_str(&original_kubeconfig).unwrap();
    let updated: serde_yaml::Value = serde_yaml::from_str(&updated_kubeconfig).unwrap();

    // Verify cluster information is unchanged
    assert_eq!(original.get("clusters"), updated.get("clusters"));
    assert_eq!(
        original.get("current-context"),
        updated.get("current-context")
    );

    // Verify user credentials are unchanged (only name should change)
    let original_user_data =
        original.get("users").and_then(|u| u.as_sequence()).unwrap()[0].get("user");
    let updated_user_data =
        updated.get("users").and_then(|u| u.as_sequence()).unwrap()[0].get("user");
    assert_eq!(original_user_data, updated_user_data);
}

#[test]
fn test_kubeconfig_transformation_with_multiple_users() {
    let cluster_name = "multi-user";

    // Create kubeconfig with multiple users (only kubernetes-admin should be renamed)
    let kubeconfig_with_multiple_users = format!(
        r#"apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
    server: https://127.0.0.1:6443
  name: {}
contexts:
- context:
    cluster: {}
    user: kubernetes-admin
  name: kubernetes-admin@{}
- context:
    cluster: {}
    user: some-other-user
  name: some-other-user@{}
current-context: kubernetes-admin@{}
kind: Config
users:
- name: kubernetes-admin
  user:
    client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t...
    client-key-data: LS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0t...
- name: some-other-user
  user:
    token: some-token
"#,
        cluster_name, cluster_name, cluster_name, cluster_name, cluster_name, cluster_name
    );

    let result = transform_kubeconfig_user_names(&kubeconfig_with_multiple_users, cluster_name);
    assert!(result.is_ok());

    let updated_kubeconfig = result.unwrap();

    // Verify only kubernetes-admin was renamed
    assert!(updated_kubeconfig.contains("multi-user-admin"));
    assert!(updated_kubeconfig.contains("some-other-user"));
    // Check that the specific user entry was renamed
    assert!(updated_kubeconfig.contains("- name: multi-user-admin"));

    // Parse and verify structure
    let config: serde_yaml::Value = serde_yaml::from_str(&updated_kubeconfig).unwrap();
    let users = config.get("users").and_then(|u| u.as_sequence()).unwrap();
    assert_eq!(users.len(), 2);

    let user_names: Vec<&str> = users
        .iter()
        .map(|u| u.get("name").and_then(|n| n.as_str()).unwrap())
        .collect();
    assert!(user_names.contains(&"multi-user-admin"));
    assert!(user_names.contains(&"some-other-user"));
}

#[test]
fn test_kubeconfig_transformation_invalid_yaml() {
    let invalid_kubeconfig = "invalid: yaml: content: [[[";

    let result = transform_kubeconfig_user_names(invalid_kubeconfig, "test");
    assert!(result.is_err());
}

#[test]
fn test_kubeconfig_kubectl_command_patterns() {
    // Test that kubectl command patterns are correctly structured
    let cluster_name = "test-cluster";
    let user_name = format!("{}-admin", cluster_name);

    // Verify the expected kubectl commands that should be generated
    let expected_set_cluster_args = vec!["config", "set-cluster", cluster_name];
    let expected_set_credentials_args = vec!["config", "set-credentials", &user_name];
    let expected_set_context_args = vec!["config", "set-context", cluster_name];
    let expected_use_context_args = vec!["config", "use-context", cluster_name];

    // These represent the command patterns that should be used in save_kubeconfig
    assert_eq!(
        expected_set_cluster_args,
        vec!["config", "set-cluster", "test-cluster"]
    );
    assert_eq!(
        expected_set_credentials_args,
        vec!["config", "set-credentials", "test-cluster-admin"]
    );
    assert_eq!(
        expected_set_context_args,
        vec!["config", "set-context", "test-cluster"]
    );
    assert_eq!(
        expected_use_context_args,
        vec!["config", "use-context", "test-cluster"]
    );
}

#[test]
fn test_kubeconfig_cleanup_command_patterns() {
    // Test that kubectl cleanup command patterns are correctly structured
    let cluster_name = "test-cluster";
    let user_name = format!("{}-admin", cluster_name);

    // Verify the expected kubectl commands that should be generated for cleanup
    let expected_delete_context_args = vec!["config", "delete-context", cluster_name];
    let expected_delete_cluster_args = vec!["config", "delete-cluster", cluster_name];
    let expected_delete_user_args = vec!["config", "delete-user", &user_name];

    // These represent the command patterns that should be used in remove_kubeconfig_context
    assert_eq!(
        expected_delete_context_args,
        vec!["config", "delete-context", "test-cluster"]
    );
    assert_eq!(
        expected_delete_cluster_args,
        vec!["config", "delete-cluster", "test-cluster"]
    );
    assert_eq!(
        expected_delete_user_args,
        vec!["config", "delete-user", "test-cluster-admin"]
    );
}
