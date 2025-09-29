# Kubelet CSR Auto-Approval in kina

## Problem

When creating Kubernetes clusters, kubelet generates Certificate Signing Requests (CSRs) for TLS certificates. While kubelet **client** certificates are auto-approved by the kube-controller-manager, kubelet **serving** certificates require manual approval.

Without serving certificates, `kubectl logs` and `kubectl exec` commands fail with TLS errors:
```
Error from server: Get "https://192.168.64.26:10250/containerLogs/kube-system/pod-name/container": remote error: tls: internal error
```

## Root Cause

- Kubernetes kube-controller-manager auto-approves `kubernetes.io/kube-apiserver-client-kubelet` CSRs
- However, `kubernetes.io/kubelet-serving` CSRs are **never auto-approved** by design
- This is a security feature to prevent compromised nodes from generating arbitrary serving certificates

## kina's Solution

kina automatically handles kubelet CSR approval during cluster creation and provides manual tools for existing clusters.

### Automatic Bootstrap (Default Behavior)

**kina automatically handles CSR approval for all new clusters by default.** When creating any cluster with `kina create`, kina will:

1. Create the cluster using Apple Container
2. Wait briefly for the API server to be ready (10 seconds minimum)
3. Monitor for kubelet-serving CSRs for 60 seconds
4. Auto-approve any kubelet-serving CSRs that appear
5. This prevents TLS errors from the start

```bash
# CSR auto-approval happens automatically for any cluster creation
kina create my-cluster

# Even with explicit wait, CSRs are still auto-approved
kina create my-cluster --wait 300
```

If CSR auto-approval fails for any reason, kina will warn you and provide the manual fix command.

### Manual Approval (Existing Clusters)

For existing clusters with TLS issues, use the manual approval command:

```bash
kina approve-csr my-cluster
```

This command:
- Finds all pending `kubernetes.io/kubelet-serving` CSRs
- Approves them immediately
- Fixes TLS errors with kubectl logs/exec

## Security Considerations

kina's auto-approval is safe because:

1. **Scope Limited**: Only approves kubelet-serving CSRs, not arbitrary certificates
2. **Node Identity**: CSRs must come from valid cluster nodes
3. **Time-Bounded**: Bootstrap approval only runs for 60 seconds after cluster creation
4. **Explicit**: Manual approval requires explicit user action

## Implementation Details

### Bootstrap Flow

1. `ClusterManager::create_cluster()` calls `bootstrap_kubelet_csrs()` after cluster is ready
2. `KubernetesClient::bootstrap_approve_kubelet_csrs()` monitors for CSRs
3. Uses `kubectl get csr` with JSONPath to find kubelet-serving CSRs
4. Approves each CSR with `kubectl certificate approve`

### Manual Approval

1. `kina approve-csr` command calls `ClusterManager::approve_kubelet_csrs()`
2. `KubernetesClient::auto_approve_kubelet_csrs()` finds pending CSRs
3. Approves all pending kubelet-serving CSRs immediately

### CSR Detection

Uses JSONPath to find kubelet-serving CSRs:
```bash
kubectl get csr -o jsonpath='{range .items[?(@.spec.signerName=="kubernetes.io/kubelet-serving")]}{.metadata.name}{"\n"}{end}'
```

## Why This Happens

1. **kubelet TLS Bootstrap**: Modern Kubernetes uses TLS bootstrapping for secure node joining
2. **Security vs Usability**: Kubernetes defaults to secure (manual approval) over convenient
3. **Local Development**: For local clusters, auto-approval is acceptable and improves UX
4. **Apple Container**: This affects all Kubernetes clusters, not just Apple Container

## Comparison with kind

- **kind**: Also has this issue but uses different solutions
- **kina**: Provides built-in auto-approval during cluster creation
- **Manual clusters**: Require either manual approval or third-party controllers

## Commands

```bash
# Create cluster (CSR auto-approval is automatic)
kina create my-cluster

# Create cluster with explicit wait (CSR auto-approval still happens)
kina create my-cluster --wait 300

# Skip CSR auto-approval if you want to handle it manually
kina create my-cluster --skip-csr-approval

# Manual approval for existing clusters or if auto-approval failed
kina approve-csr my-cluster

# Check cluster status (will show CSR-related issues)
kina status my-cluster --verbose
```

## Related Files

- `kina-cli/src/core/kubernetes.rs`: CSR approval implementation
- `kina-cli/src/core/cluster.rs`: Bootstrap integration
- `kina-cli/src/cli/cluster.rs`: CLI commands