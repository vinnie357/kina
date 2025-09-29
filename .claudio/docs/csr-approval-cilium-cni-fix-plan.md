# CSR Auto-Approval and Cilium CNI Fix Plan for kina CLI

**Date**: 2025-09-27
**Test Execution**: Single-Node Cluster Integration Test
**Issues**: CSR Auto-Approval Failure + Cilium CNI Apple Container Incompatibility

## Executive Summary

The kina CLI successfully creates Kubernetes clusters using Apple Container, but two critical issues prevent functional workload deployment:

1. **CSR Auto-Approval Failure**: kubelet certificates not automatically approved
2. **CNI Networking Incompatibility**: Cilium crashes due to Apple Container networking limitations

## Issue Analysis

### Issue #1: CSR Auto-Approval Bug (Priority: HIGH)

**Problem**:
- kubelet-serving Certificate Signing Requests remain in Pending state
- Causes TLS errors when accessing kubelet API (port 10250)
- Prevents log retrieval and proper cluster monitoring

**Root Cause**:
```
WARN No kubelet-serving CSRs were found during bootstrap period
```

**Impact**:
- Manual intervention required: `kubectl certificate approve <csr-name>`
- Cluster appears functional but kubelet API inaccessible
- Diagnostic capabilities severely limited

**Location**: `kina-cli/src/core/kubernetes.rs:46` - CSR detection/approval logic

### Issue #2: Cilium CNI Incompatibility (Priority: CRITICAL)

**Problem**:
- Cilium agent crashes with: `address family not supported by protocol`
- Error occurs when setting IPv4 routing rules
- Node remains tainted: `node.cilium.io/agent-not-ready`

**Root Cause**:
```
error while initializing daemon: ensuring local routing rule:
replace local ipv4 rule: address family not supported by protocol
```

**Impact**:
- No pod scheduling possible
- Cluster unusable for workloads
- CNI networking completely non-functional

**Location**: Apple Container networking incompatibility with Cilium's routing requirements

## Fix Plan

### Phase 1: CSR Auto-Approval Fix (Immediate - 1-2 days)

#### 1.1 Investigate CSR Detection Logic
**File**: `kina-cli/src/core/kubernetes.rs`
**Current Issue**: CSR detection timeout or filtering problem

**Investigation Steps**:
```rust
// Check current CSR detection in bootstrap_kubelet_csr_auto_approval
// Line ~206: pub async fn bootstrap_kubelet_csr_auto_approval

// Expected behavior:
// 1. Watch for kubelet-serving CSRs
// 2. Auto-approve valid requests
// 3. Continue until timeout or success
```

**Debug Actions**:
1. Add detailed logging to CSR detection loop
2. Verify CSR watch filters and selectors
3. Check timing - CSRs may appear after bootstrap timeout
4. Test manual approval to confirm fix approach

#### 1.2 Implement Enhanced CSR Approval
**Changes Required**:
1. **Extended Timeout**: Increase CSR watch timeout from 60s to 120s
2. **Improved Filtering**: Better CSR detection for kubelet-serving requests
3. **Retry Logic**: Multiple attempts with backoff
4. **Validation**: Verify CSR approval success

**Implementation**:
```rust
// Enhanced CSR approval in kubernetes.rs
async fn bootstrap_kubelet_csr_auto_approval_enhanced(
    &self,
    kubeconfig_path: &str,
    timeout: Duration
) -> Result<()> {
    // 1. Extended timeout loop
    // 2. Better CSR filtering
    // 3. Approval verification
    // 4. Detailed logging
}
```

#### 1.3 Testing
- Create test cluster
- Verify automatic CSR approval
- Confirm kubelet API access: `kubectl logs -n kube-system <pod>`

### Phase 2: CNI Solution Research (2-3 days)

#### 2.1 CNI Compatibility Analysis
**Objective**: Determine Apple Container networking constraints

**Research Areas**:
1. **Apple Container Networking Model**
   - Available networking operations
   - Route table management capabilities
   - IPTables/netfilter support
   - Bridge/veth interface support

2. **CNI Requirements Analysis**
   - Cilium networking requirements
   - Alternative CNI options (Flannel, Calico, etc.)
   - Minimal CNI implementation requirements

#### 2.2 CNI Alternative Evaluation

**Option A: Flannel CNI**
- **Pros**: Simpler networking model, VXLAN overlay
- **Cons**: May still have routing rule issues
- **Compatibility**: Better chance with Apple Container

**Option B: Host-Only Networking**
- **Pros**: Minimal CNI requirements
- **Cons**: Limited multi-node capabilities
- **Use Case**: Single-node development clusters

**Option C: Custom CNI Bridge**
- **Pros**: Tailored for Apple Container
- **Cons**: Significant development effort
- **Approach**: Bridge-based networking without complex routing

**Option D: Cilium Configuration Tuning**
- **Pros**: Keep existing CNI
- **Cons**: May not be possible with Apple Container limitations
- **Approach**: Disable problematic features

#### 2.3 Implementation Decision Matrix

| CNI Option | Effort | Compatibility | Features | Recommendation |
|------------|--------|---------------|----------|----------------|
| Flannel | Low | Medium | Basic | Test First |
| Host-Only | Very Low | High | Limited | Quick Fix |
| Custom CNI | High | High | Full | Long-term |
| Cilium Tuned | Medium | Low | Full | Research |

### Phase 3: Implementation (3-5 days)

#### 3.1 CSR Fix Implementation
**Timeline**: 1-2 days
**Files**:
- `kina-cli/src/core/kubernetes.rs`
- `kina-cli/src/core/cluster.rs`

**Steps**:
1. Implement enhanced CSR approval logic
2. Add comprehensive logging
3. Test with cluster creation
4. Verify kubelet API access

#### 3.2 CNI Integration
**Timeline**: 2-3 days (depending on chosen solution)

**Flannel Implementation** (Recommended First):
```rust
// In apple_container.rs - replace Cilium installation
async fn install_flannel_cni(&self, container_id: &str) -> KinaResult<()> {
    // 1. Apply Flannel YAML manifests
    // 2. Configure for single-node
    // 3. Verify networking
}
```

**Files to Modify**:
- `kina-cli/src/core/apple_container.rs` (line ~967: Cilium installation)
- Add Flannel manifests to `kina-cli/manifests/`
- Update cluster validation logic

#### 3.3 Testing and Validation
**Integration Tests**:
1. Cluster creation with new CNI
2. Pod scheduling and networking
3. Service connectivity
4. Clean cluster deletion

### Phase 4: Test Plan Updates (1 day)

#### 4.1 Update Test Plan Document
**File**: `.claudio/docs/single-node-cluster-test-plan.md`

**Updates**:
1. Add CNI readiness validation steps
2. Include CSR approval verification
3. Add troubleshooting section
4. Update expected timing for cluster readiness

#### 4.2 Add CNI-Specific Tests
**New Test Cases**:
- WF-005: CNI Plugin Validation
- WF-006: Pod Networking Tests
- WF-007: Service Discovery Tests

## Implementation Priority

### Immediate (Next 1-2 days)
1. **Fix CSR Auto-Approval** - Critical for cluster usability
2. **Research Flannel CNI** - Prepare alternative solution

### Short-term (Next week)
1. **Implement Flannel CNI** - Replace Cilium
2. **Comprehensive Testing** - Validate full workflow
3. **Update Documentation** - Test plan and troubleshooting

### Medium-term (Next 2 weeks)
1. **Optimize CNI Performance** - Fine-tune networking
2. **Add Multi-Node Support** - If required
3. **Enhanced Monitoring** - Better cluster health checks

## Success Criteria

### Phase 1 Success:
- [ ] CSRs automatically approved during cluster creation
- [ ] `kubectl logs` works without manual intervention
- [ ] kubelet API accessible on port 10250

### Phase 2 Success:
- [ ] CNI plugin starts successfully
- [ ] Node reaches Ready state
- [ ] Pods can be scheduled and reach Running state

### Phase 3 Success:
- [ ] Complete cluster lifecycle (create → use → delete) works
- [ ] Pod-to-pod networking functional
- [ ] Service discovery working
- [ ] Clean resource cleanup

## Risk Assessment

### High Risk:
- **Apple Container Networking Limitations**: May require significant CNI customization
- **Unknown Compatibility Issues**: Other networking problems may emerge

### Medium Risk:
- **Performance Impact**: Alternative CNI may be slower than Cilium
- **Feature Limitations**: Some Kubernetes networking features may not work

### Low Risk:
- **CSR Approval Fix**: Well-understood problem with clear solution
- **Test Plan Updates**: Straightforward documentation updates

## Resource Requirements

### Development Time:
- **CSR Fix**: 8-16 hours
- **CNI Research**: 16-24 hours
- **CNI Implementation**: 24-40 hours
- **Testing & Documentation**: 8-16 hours
- **Total**: 56-96 hours (7-12 days)

### Testing Requirements:
- Multiple test cluster creations
- Various workload deployments
- Network connectivity validation
- Performance benchmarking

## Next Actions

1. **Immediate**: Start CSR auto-approval investigation
2. **Day 1**: Begin Flannel CNI research
3. **Day 2**: Implement CSR fix and test
4. **Day 3-4**: Implement Flannel CNI replacement
5. **Day 5**: Comprehensive testing and validation
6. **Day 6**: Documentation updates and test plan revision

This plan addresses the critical networking issues that prevent kina from creating functional Kubernetes clusters, with a focus on quick wins (CSR fix) followed by the more complex CNI replacement.