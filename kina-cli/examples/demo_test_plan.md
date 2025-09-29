# Kina Demo Test Plan

This document outlines the complete test plan for the Kina demo workflow using mise commands.

## Overview

The demo workflow demonstrates Kina's ability to:
- Create Kubernetes clusters using Apple Container runtime
- Install nginx-ingress (nginx.org version) with CRDs
- Deploy demo applications with ingress routing
- Provide working examples for users

## Prerequisites

- macOS 15.6+ with Apple Container runtime
- kubectl installed and in PATH
- mise installed and configured
- Kina project cloned with mise.toml configured

## Test Commands

### 1. Demo Cluster Creation
```bash
mise run demo-cluster
```

**Expected Behavior:**
- ✅ Generate unique cluster name with timestamp: `demo-YYYYMMDD-HHMMSS`
- ✅ Build kina CLI successfully
- ✅ Create cluster with control plane node
- ✅ Install nginx-ingress (nginx.org) with all manifests:
  - ns-and-sa.yaml (namespace and ServiceAccount)
  - rbac.yaml (RBAC resources)
  - crds.yaml (Custom Resource Definitions)
  - nginx-config.yaml (default configuration)
  - ingress-class.yaml (IngressClass)
  - nginx-ingress-daemonset.yaml (DaemonSet deployment)
- ✅ Wait for nginx-ingress pods to be ready
- ✅ Deploy demo application with beautiful UI
- ✅ Create ingress with nginx.org annotations
- ✅ Test ingress routing
- ✅ Provide access URL and instructions

**Success Criteria:**
- Cluster creates without errors
- nginx-ingress pods show "1/1 Running"
- Demo app responds to curl with correct title
- Access instructions display cluster IP

### 2. Demo Testing
```bash
mise run demo-test
```

**Expected Behavior:**
- ✅ Find latest demo cluster automatically
- ✅ Test cluster connectivity
- ✅ Verify nginx-ingress is running
- ✅ Test demo app via ingress routing
- ✅ Validate all components are functional
- ✅ Display comprehensive test results

**Success Criteria:**
- All connectivity tests pass
- Ingress routing returns correct response
- Demo app title: "🎉 Kina Demo App"
- No error messages in test output

### 3. Demo Cleanup
```bash
mise run demo-cleanup
```

**Expected Behavior:**
- ✅ Find all demo clusters (prefix: `demo-`)
- ✅ Display list of clusters to be cleaned
- ✅ Confirm deletion with user
- ✅ Delete all demo clusters
- ✅ Remove kubeconfig files
- ✅ Clean up any leftover resources

**Success Criteria:**
- All demo clusters removed from `container list`
- Kubeconfig files removed from `~/.kube/`
- Clean exit with summary message

## Manual Verification Steps

### Pre-Demo Verification
```bash
# Check prerequisites
container version
kubectl version --client
mise --version

# Verify no existing demo clusters
container list | grep demo
```

### Post-Demo Verification
```bash
# 1. Verify cluster is running
CLUSTER_NAME=$(container list | grep demo | head -1 | awk '{print $1}' | sed 's/-control-plane//')
container list | grep $CLUSTER_NAME

# 2. Check nginx-ingress pods
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get pods -n nginx-ingress

# 3. Verify ingress configuration
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get ingress kina-demo-ingress
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME describe ingress kina-demo-ingress

# 4. Test demo app
CLUSTER_IP=$(container list | grep "$CLUSTER_NAME-control-plane" | awk '{print $NF}')
curl -H "Host: demo.kina.local" http://$CLUSTER_IP

# 5. Verify nginx.org (not ingress-nginx)
kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get ingressclass
# Should show: nginx   nginx.org/ingress-controller   <none>   XXm
```

## Test Scenarios

### Scenario 1: Fresh Environment
1. Clean system with no existing clusters
2. Run `mise run demo-cluster`
3. Verify all components
4. Run `mise run demo-test`
5. Run `mise run demo-cleanup`

### Scenario 2: Multiple Demo Clusters
1. Run `mise run demo-cluster` multiple times
2. Verify each creates unique cluster name
3. Run `mise run demo-test` (should test latest)
4. Run `mise run demo-cleanup` (should clean all)

### Scenario 3: Error Recovery
1. Interrupt demo-cluster during creation
2. Re-run demo-cluster
3. Verify it handles partial state gracefully

## Common Issues and Solutions

### Issue: nginx-ingress Installation Fails
**Symptoms:** Error during Step 3 of demo-cluster
**Solution:**
- Verify manifests directory exists: `ls kina-cli/manifests/nginx-ingress/`
- Check working directory in mise script
- Ensure Cargo.toml is in current directory

### Issue: Ingress Returns 404
**Symptoms:** curl returns `<title>404 Not Found</title>`
**Solution:**
- Check ingress annotations (should use nginx.org/, not kubernetes.io/)
- Verify IngressClass: `nginx.org/ingress-controller`
- Check ingress events for errors

### Issue: Demo App Not Accessible
**Symptoms:** Connection refused or timeout
**Solution:**
- Verify cluster IP: `container list`
- Check demo app pods: `kubectl get pods -l app=kina-demo-app`
- Test service directly: `kubectl port-forward svc/kina-demo-service 8080:80`

### Issue: Cleanup Doesn't Find Clusters
**Symptoms:** "No demo clusters found"
**Solution:**
- Check cluster naming: `container list | grep demo`
- Verify grep pattern in cleanup script
- Manual cleanup: `container stop <cluster-name> && container rm <cluster-name>`

## Expected Test Output

### Successful Demo-Cluster Output
```
🎉 KINA DEMO CLUSTER SETUP
========================================
Cluster Name: demo-20240928-123456

✅ Step 1: Building kina CLI...
✅ Step 2: Creating cluster 'demo-20240928-123456'...
✅ Step 3: Installing nginx-ingress controller...
✅ Step 4: Waiting for nginx-ingress to be ready...
✅ Step 5: Deploying demo application...
✅ Step 6: Creating ingress configuration...
✅ Step 7: Testing demo application...

🎉 Demo Complete! Access your app at:
   curl -H 'Host: demo.kina.local' http://192.168.64.XXX

Or add to /etc/hosts:
   192.168.64.XXX demo.kina.local
   Then visit: http://demo.kina.local
```

### Successful Demo-Test Output
```
🧪 Testing demo cluster: demo-20240928-123456
========================================

✅ Cluster connectivity: OK
✅ nginx-ingress status: Running (1/1)
✅ Demo app pods: Ready (2/2)
✅ Ingress routing: Working
✅ Demo app response: 🎉 Kina Demo App

🎯 All tests passed! Demo is fully functional.
```

### Successful Demo-Cleanup Output
```
🧹 Finding demo clusters to clean up...

Found demo clusters:
demo-20240928-123456
demo-20240928-124512

Cleaning up demo clusters...
✅ Deleted cluster: demo-20240928-123456
✅ Deleted cluster: demo-20240928-124512
✅ Removed kubeconfig files

🎉 Cleanup complete! 2 clusters removed.
```

## Performance Expectations

- **demo-cluster**: 2-3 minutes (including cluster bootstrap)
- **demo-test**: 10-15 seconds
- **demo-cleanup**: 30-60 seconds per cluster

## Troubleshooting Quick Reference

```bash
# Check cluster status
container list

# Check nginx-ingress logs
kubectl --kubeconfig ~/.kube/<cluster> logs -n nginx-ingress -l app=nginx-ingress

# Debug ingress
kubectl --kubeconfig ~/.kube/<cluster> describe ingress kina-demo-ingress

# Manual test
CLUSTER_IP=$(container list | grep "<cluster>-control-plane" | awk '{print $NF}')
curl -v -H "Host: demo.kina.local" http://$CLUSTER_IP

# Force cleanup
container list | grep demo | awk '{print $1}' | xargs -I {} sh -c 'container stop {} && container rm {}'
```

## Validation Checklist

- [ ] demo-cluster creates cluster successfully
- [ ] nginx-ingress (nginx.org) installs with all CRDs
- [ ] Demo app deploys and is accessible
- [ ] Ingress uses correct nginx.org annotations
- [ ] demo-test validates all components
- [ ] demo-cleanup removes all demo resources
- [ ] No leftover containers or kubeconfig files
- [ ] All commands work in fresh environment
- [ ] Error messages are clear and actionable