#!/bin/bash
set -euo pipefail

echo "Applying Cilium Apple Container compatibility patch..."

# Apply the patch
kubectl patch configmap cilium-config -n kube-system --patch-file kina-cli/manifests/cilium-apple-container-patch.yaml

# Restart Cilium pods to pick up new config
echo "Restarting Cilium pods..."
kubectl delete pods -n kube-system -l k8s-app=cilium

# Wait for pods to restart
echo "Waiting for Cilium pods to be ready..."
kubectl wait --for=condition=ready pod -l k8s-app=cilium -n kube-system --timeout=300s

echo "Cilium patch applied successfully"

# Check status
echo "Checking Cilium status..."
kubectl get pods -n kube-system -l k8s-app=cilium