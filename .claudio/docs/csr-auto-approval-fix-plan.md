# CSR Auto-Approval Fix Plan for kina CLI

**Date**: 2025-09-27
**Issue**: kubelet Certificate Signing Requests not automatically approved
**Priority**: HIGH - Blocking cluster diagnostics and functionality validation

## Problem Statement

The kina CLI creates Kubernetes clusters successfully, but kubelet-serving Certificate Signing Requests (CSRs) remain in Pending state, causing:

- TLS errors when accessing kubelet API (port 10250)
- `kubectl logs` failures
- Inability to diagnose other cluster issues (including CNI problems)
- Manual intervention required: `kubectl certificate approve <csr-name>`

## Root Cause Analysis

### Observed Behavior
```
WARN No kubelet-serving CSRs were found during bootstrap period
```

### Current Implementation
**File**: `kina-cli/src/core/kubernetes.rs`
**Function**: `bootstrap_kubelet_csr_auto_approval` (line ~206)

### Potential Root Causes
1. **Timing Issue**: CSRs appear after bootstrap timeout expires
2. **Filtering Problem**: CSR detection logic not finding kubelet-serving CSRs
3. **Watch Logic Error**: Kubernetes watch not triggering on CSR creation
4. **Approval Logic Failure**: CSRs detected but approval failing

## Investigation Plan

### Step 1: Confirm Issue Reproducibility ✅ COMPLETED
```bash
# Test current behavior
mise run kina create debug-cluster-001
kubectl get csr  # Should show pending kubelet-serving CSRs
kubectl certificate approve <csr-names>  # Manual fix
kubectl logs -n kube-system <any-pod>  # Should work after approval
mise run kina delete debug-cluster-001
```

**Status**: Issue confirmed - CSRs remain pending, manual approval fixes kubelet API access

### Step 2: Add Debug Logging ✅ COMPLETED
**File**: `kina-cli/src/core/kubernetes.rs`
**Location**: `bootstrap_approve_kubelet_csrs` function

**Changes Made**:
- ✅ Added comprehensive emoji-based logging for better readability
- ✅ Added initial CSR state listing with `list_all_csrs` helper function
- ✅ Enhanced error reporting with specific failure reasons
- ✅ Added progress logging every 10 seconds
- ✅ Added final CSR state listing when no CSRs found
- ✅ Added detailed iteration tracking and timing information

**Status**: Enhanced logging implemented and compiled successfully

### Step 3: Analyze CSR Timing ✅ COMPLETED
**Root Cause Identified**: Kubeconfig path mismatch

**Issue**: CSR auto-approval function uses wrong kubeconfig path:
- **Expected**: `/Users/vinnie/.kube/csr-debug-cluster-001`
- **Actual**: `/Users/vinnie/Library/Application Support/dev.kina.kina/kubeconfig/csr-debug-cluster-001.yaml`

**Evidence**:
- Cluster created successfully, kubeconfig saved to correct location
- CSR auto-approval process cannot access cluster due to wrong path
- Manual check reveals 2 pending kubelet-serving CSRs: `csr-lnrhx` and `csr-wdpj4`
- 1 kube-apiserver-client-kubelet CSR automatically approved: `csr-hz5dx`

**Status**: Root cause identified - kubeconfig path resolution bug in cluster creation

### Step 4: Test CSR Detection Logic
```rust
// Test CSR listing and filtering
let csrs = client.list(&ListParams::default()).await?;
for csr in &csrs {
    info!("Found CSR: {} status: {:?} usage: {:?}",
          csr.metadata.name, csr.status, csr.spec.usages);
}
```

## Fix Implementation

### Phase 1: Enhanced Logging and Debugging (4-8 hours)

**Changes to `kina-cli/src/core/kubernetes.rs`**:

```rust
pub async fn bootstrap_kubelet_csr_auto_approval_debug(
    &self,
    kubeconfig_path: &str,
    timeout: Duration,
) -> Result<()> {
    info!("🔐 Starting enhanced kubelet CSR auto-approval");
    info!("⏱️  Timeout: {}s", timeout.as_secs());

    let start_time = Instant::now();
    let client: Api<CertificateSigningRequest> = Api::all(self.client.clone());

    // List existing CSRs first
    let existing_csrs = client.list(&ListParams::default()).await?;
    info!("📋 Found {} existing CSRs", existing_csrs.items.len());
    for csr in &existing_csrs.items {
        info!("   CSR: {} - Status: {:?}",
              csr.metadata.name.as_ref().unwrap_or(&"<unknown>".to_string()),
              csr.status.as_ref().map(|s| &s.conditions).unwrap_or(&vec![]));
    }

    // Enhanced watch with detailed logging
    let watcher = watcher(client.clone(), watcher::Config::default());
    pin_mut!(watcher);

    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= timeout {
            warn!("❌ CSR auto-approval timeout after {}s", elapsed.as_secs());
            break;
        }

        // Log progress every 10 seconds
        if elapsed.as_secs() % 10 == 0 {
            info!("⏳ CSR watch progress: {}s / {}s", elapsed.as_secs(), timeout.as_secs());
        }

        // Enhanced event handling with detailed logging
        match tokio::time::timeout(Duration::from_secs(5), watcher.try_next()).await {
            Ok(Ok(Some(event))) => {
                match event {
                    watcher::Event::Applied(csr) => {
                        info!("🔍 CSR Event: {}", csr.metadata.name.as_ref().unwrap_or(&"<unknown>".to_string()));
                        // ... rest of approval logic with detailed logging
                    }
                    _ => {}
                }
            }
            Ok(Ok(None)) => {
                warn!("🚫 CSR watch stream ended unexpectedly");
                break;
            }
            Ok(Err(e)) => {
                error!("❌ CSR watch error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout on individual watch - continue loop
                continue;
            }
        }
    }

    Ok(())
}
```

### Phase 2: Fix Implementation (4-8 hours)

Based on debug findings, implement the specific fix:

**Option A: Extended Timeout**
```rust
// Increase timeout from 60s to 120s
let timeout = Duration::from_secs(120);
```

**Option B: Improved CSR Detection**
```rust
// Better filtering for kubelet-serving CSRs
fn is_kubelet_serving_csr(csr: &CertificateSigningRequest) -> bool {
    csr.spec.usages.contains(&"digital signature".to_string()) &&
    csr.spec.usages.contains(&"key encipherment".to_string()) &&
    csr.spec.usages.contains(&"server auth".to_string()) &&
    csr.metadata.name.as_ref()
        .map(|name| name.starts_with("csr-"))
        .unwrap_or(false)
}
```

**Option C: Retry Logic**
```rust
// Multiple approval attempts
for attempt in 1..=3 {
    match self.approve_csr(&csr_name).await {
        Ok(_) => {
            info!("✅ CSR {} approved on attempt {}", csr_name, attempt);
            break;
        }
        Err(e) => {
            warn!("⚠️  CSR approval attempt {} failed: {}", attempt, e);
            if attempt < 3 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
```

### Phase 3: Testing and Validation (2-4 hours)

**Test Plan**:
1. Create cluster with debug logging
2. Monitor CSR events in real-time
3. Verify automatic approval
4. Confirm kubelet API access
5. Test log retrieval functionality

**Validation Commands**:
```bash
# Full test cycle
mise run kina create csr-test-cluster
kubectl get csr  # Should show approved CSRs
kubectl logs -n kube-system cilium-<pod>  # Should work without errors
kubectl cluster-info dump  # Should work
mise run kina delete csr-test-cluster
```

## Success Criteria ✅ COMPLETED

- [x] CSRs automatically approved during cluster creation
- [x] No manual `kubectl certificate approve` required
- [x] `kubectl logs` works for all system pods
- [x] kubelet API accessible on port 10250
- [x] Cluster creation logs show successful CSR approval

## Final Resolution ✅ COMPLETED

### Root Cause: Kubeconfig Path Mismatch
The CSR auto-approval function was using the wrong kubeconfig path. Apple Container saves kubeconfig files to `~/.kube/{cluster_name}`, but the CSR approval function was looking in the application support directory.

### Fix Implementation
**File**: `kina-cli/src/core/cluster.rs:256-281`

**Change**: Updated `bootstrap_kubelet_csrs` function to use correct kubeconfig path:
```rust
// Get kubeconfig path where Apple Container actually saves it (~/.kube/{cluster_name})
let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
let kube_dir = std::path::Path::new(&home_dir).join(".kube");
let kubeconfig_path = kube_dir.join(cluster_name);
```

**Previous path**: `/Users/vinnie/Library/Application Support/dev.kina.kina/kubeconfig/{cluster_name}.yaml`
**Corrected path**: `/Users/vinnie/.kube/{cluster_name}`

### Validation Results
**Test Cluster**: `csr-validation-test`
**Results**:
- ✅ **24 CSRs automatically approved** during cluster creation
- ✅ kubelet API access working (kubectl logs successful)
- ✅ Enhanced logging shows real-time CSR approval progress
- ✅ No manual intervention required

**Test Output Sample**:
```
🔐 Starting enhanced kubelet CSR bootstrap auto-approval
⏱️  Timeout: 60s
📋 Kubeconfig: /Users/vinnie/.kube/csr-validation-test
📊 Initial CSR state: 3 total CSRs found
✅ Found 2 pending kubelet-serving CSRs
✅ Successfully approved CSR: csr-2nn68
✅ Successfully approved CSR: csr-69sqh
✅ Bootstrap CSR approval completed: 24 CSRs approved
```

### Impact Assessment
This fix resolves **Issue #1: CSR Auto-Approval Failure** completely. With kubelet API access now working, we can:
- Properly diagnose cluster issues using `kubectl logs`
- Access kubelet metrics and health endpoints
- Proceed with investigating **Issue #2: Cilium CNI compatibility** with proper visibility

**Status**: ✅ **RESOLVED** - CSR auto-approval working correctly

## Implementation Timeline

| Phase | Duration | Tasks |
|-------|----------|-------|
| Debug | 4-8 hours | Add logging, reproduce issue, analyze root cause |
| Fix | 4-8 hours | Implement solution based on findings |
| Test | 2-4 hours | Validate fix, update test plan |
| **Total** | **10-20 hours** | **Complete CSR auto-approval fix** |

## Next Steps After CSR Fix ✅ COMPLETED

✅ **CSR auto-approval works** - Successfully implemented and tested

### Issue #2: Cilium CNI Compatibility ⚠️ CONFIRMED SEPARATE ISSUE
Testing with the fixed CSR auto-approval confirms that **Issue #2 is a distinct Apple Container networking incompatibility**:

**Observation**: Even with working kubelet API access, Cilium pods remain stuck in `Init:0/6` state for extended periods, indicating fundamental networking compatibility issues between Cilium and Apple Container's networking model.

**Status**: CSR Issue #1 completely resolved, Issue #2 requires separate investigation focusing on CNI alternatives compatible with Apple Container.

### Recommended Next Actions
1. ✅ **CSR Fix**: Successfully implemented and validated
2. 🔄 **CNI Investigation**: Focus on Flannel or host-only networking alternatives
3. ✅ **Test Plan Updates**: CSR approval verification now included
4. ✅ **Documentation**: CSR troubleshooting guide updated

The CSR auto-approval fix is complete and working reliably. The Cilium CNI compatibility issue is confirmed as a separate technical challenge requiring alternative networking solutions.