# Default CNI Solution for kina 15.6+ (Apple Container VMs)

## Problem Summary

Apple Container VMs use kata-containers kernels that have `CONFIG_BRIDGE_NETFILTER` disabled, causing standard CNI plugins (Cilium, Flannel) to fail with bridge netfilter errors.

## Root Cause Analysis

### Kernel Limitations
- **Source**: Apple Container uses kernels from [kata-containers project](https://github.com/kata-containers/kata-containers)
- **Missing Module**: `CONFIG_BRIDGE_NETFILTER is not set` in all tested kernel configurations
- **Impact**: Bridge-based CNI plugins cannot function without bridge netfilter support

### Failed CNI Attempts
1. **Cilium**: Failed with bridge netfilter and eBPF limitations
2. **Flannel**: Failed with same bridge netfilter requirements
3. **Bridge CNI**: Missing bridge binary in standard CNI installation

## Solution: PTP CNI with Host-Local IPAM

### Why PTP CNI Works
- **No Bridge Dependency**: Uses point-to-point veth pairs instead of bridge networks
- **Kernel Compatible**: Only requires basic networking features available in kata-containers kernels
- **Simple Routing**: Direct host routes to each pod via dedicated veth interfaces

### Configuration
```json
{
  "cniVersion": "0.4.0",
  "name": "ptp-net",
  "plugins": [
    {
      "type": "ptp",
      "ipMasq": true,
      "ipam": {
        "type": "host-local",
        "subnet": "10.244.0.0/16",
        "routes": [
          { "dst": "0.0.0.0/0" }
        ]
      }
    },
    {
      "type": "portmap",
      "capabilities": {
        "portMappings": true
      }
    }
  ]
}
```

## Network Architecture

### Pod Networking Model
- **Host Interface**: veth pairs with /32 addresses (e.g., `10.244.0.1/32`)
- **Pod Interface**: Individual IP addresses from subnet (e.g., `10.244.0.5/24`)
- **Routing**: Direct host routes for each pod IP

### Example Network Layout
```
Host: 192.168.64.178
├── veth876c9a7d@if2 → 10.244.0.1/32 → Pod: 10.244.0.2
├── vetha3846dd8@if2 → 10.244.0.1/32 → Pod: 10.244.0.3
├── vethe2b2c053@if2 → 10.244.0.1/32 → Pod: 10.244.0.5
└── veth06224e2d@if2 → 10.244.0.1/32 → Pod: 10.244.0.6
```

### Routing Table
```
10.244.0.2 dev veth876c9a7d scope host
10.244.0.3 dev vetha3846dd8 scope host
10.244.0.5 dev vethe2b2c053 scope host
10.244.0.6 dev veth06224e2d scope host
```

## Validation Results

### ✅ Working Features
- **Pod Creation**: Pods get IP addresses from 10.244.0.0/16
- **Pod-to-Pod Communication**: HTTP requests work between pods
- **Service Discovery**: ClusterIP services function correctly
- **External Access**: NodePort services accessible from host
- **kubectl Operations**: logs, exec, port-forward after CSR approval
- **DNS Resolution**: CoreDNS pods operational

### ✅ Test Results
```bash
# Pod-to-pod HTTP connectivity
kubectl exec test-pod -- wget -qO- http://10.244.0.7
# Returns: nginx welcome page

# External NodePort access
curl http://192.168.64.178:31399
# Returns: nginx welcome page

# Pod networking
kubectl get pods -o wide
# Shows: pods with 10.244.0.x IPs, all Ready
```

## Additional Requirements

### Certificate Signing Request (CSR) Approval
kubelet serving certificates require manual approval for kubectl logs/exec:

```bash
# Check pending CSRs
kubectl get csr

# Approve kubelet serving certificates
kubectl certificate approve <csr-name>

# Restart kubelet to use new certificates
systemctl restart kubelet
```

## Implementation in kina-cli

### Default CNI Configuration
- **Location**: `/etc/cni/net.d/10-ptp.conflist`
- **Plugin**: PTP with host-local IPAM
- **Subnet**: `10.244.0.0/16` (Kubernetes standard)
- **Automatic**: No manual intervention required

### Cluster Creation Process
1. Create Apple Container VM
2. Initialize Kubernetes with kubeadm
3. Deploy PTP CNI configuration
4. Restart kubelet
5. Auto-approve kubelet CSRs
6. Verify cluster ready state

## Benefits Over Bridge CNI

- **Kernel Compatibility**: Works with kata-containers kernel limitations
- **Simplicity**: No bridge network configuration required
- **Performance**: Direct point-to-point networking
- **Reliability**: Fewer kernel dependencies
- **Debugging**: Clear 1:1 veth to pod mapping

## Limitations

- **Single Node Only**: PTP CNI doesn't support multi-node clusters
- **No Network Policies**: Limited security features compared to Cilium
- **Manual CSR Approval**: kubelet certificates need approval for full functionality

## Conclusion

PTP CNI provides a robust, kernel-compatible networking solution for Apple Container-based Kubernetes clusters, successfully resolving the bridge netfilter limitations while maintaining full pod networking functionality.