#!/bin/bash
set -euo pipefail

echo "Restoring Cilium to full eBPF configuration..."

# Remove any existing Cilium installation
echo "Cleaning up existing Cilium installation..."
kubectl delete -f kina-cli/manifests/cilium.yaml --ignore-not-found=true

# Wait for cleanup
echo "Waiting for cleanup to complete..."
sleep 10

# Apply the original Cilium configuration with full eBPF support
echo "Applying Cilium with full eBPF support..."
kubectl apply -f kina-cli/manifests/cilium.yaml

# Wait for Cilium to be ready
echo "Waiting for Cilium pods to be ready..."
kubectl wait --for=condition=ready pod -l k8s-app=cilium -n kube-system --timeout=300s

echo "Checking Cilium status..."
kubectl get pods -n kube-system -l k8s-app=cilium

# Show Cilium agent logs to verify eBPF functionality
echo "Checking Cilium agent logs..."
kubectl logs -n kube-system -l k8s-app=cilium --tail=20