#!/bin/bash

# NGINX Ingress Examples Deployment Script for Kina
#
# This script helps deploy and test the ingress examples quickly.
# Usage: ./deploy-examples.sh [example-name] [cluster-name]

set -euo pipefail

# Default values
CLUSTER_NAME="${2:-test-cluster}"
KUBECONFIG_PATH="$HOME/.kube/$CLUSTER_NAME"
CLUSTER_IP=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."

    # Check if kina is available
    if ! command -v kina &> /dev/null; then
        error "kina CLI not found. Please ensure kina is built and in your PATH."
    fi

    # Check if kubectl is available
    if ! command -v kubectl &> /dev/null; then
        error "kubectl not found. Please install kubectl."
    fi

    # Check if cluster exists
    if ! kina list | grep -q "$CLUSTER_NAME"; then
        error "Cluster '$CLUSTER_NAME' not found. Create it with: kina create $CLUSTER_NAME"
    fi

    # Check if kubeconfig exists
    if [[ ! -f "$KUBECONFIG_PATH" ]]; then
        error "Kubeconfig not found at $KUBECONFIG_PATH"
    fi

    # Get cluster IP
    CLUSTER_IP=$(container list | grep "$CLUSTER_NAME-control-plane" | awk '{print $NF}' || true)
    if [[ -z "$CLUSTER_IP" ]]; then
        error "Could not determine cluster IP for $CLUSTER_NAME"
    fi

    log "Using cluster: $CLUSTER_NAME ($CLUSTER_IP)"
}

# Check nginx-ingress status
check_nginx_ingress() {
    log "Checking nginx-ingress controller..."

    if ! kubectl --kubeconfig="$KUBECONFIG_PATH" get pods -n nginx-ingress &> /dev/null; then
        error "nginx-ingress namespace not found. Install with: kina install nginx-ingress --cluster $CLUSTER_NAME"
    fi

    local ready_pods
    ready_pods=$(kubectl --kubeconfig="$KUBECONFIG_PATH" get pods -n nginx-ingress --no-headers | awk '$2 ~ /1\/1/ && $3 == "Running"' | wc -l)

    if [[ $ready_pods -eq 0 ]]; then
        error "No ready nginx-ingress pods found. Check with: kubectl --kubeconfig ~/.kube/$CLUSTER_NAME get pods -n nginx-ingress"
    fi

    log "nginx-ingress controller is ready ($ready_pods pods)"
}

# Deploy an example
deploy_example() {
    local example="$1"
    local example_file="$example.yaml"

    if [[ ! -f "$example_file" ]]; then
        error "Example file '$example_file' not found"
    fi

    log "Deploying $example example..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" apply -f "$example_file"

    log "Waiting for pods to be ready..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" wait --for=condition=Ready pods --all --timeout=60s

    log "✅ $example deployed successfully"
}

# Test an example
test_example() {
    local example="$1"

    log "Testing $example..."

    case "$example" in
        "basic-web-app")
            test_basic_web_app
            ;;
        "multi-service-routing")
            test_multi_service_routing
            ;;
        "virtual-hosts")
            test_virtual_hosts
            ;;
        *)
            warn "No specific test available for $example. Use manual testing."
            ;;
    esac
}

test_basic_web_app() {
    log "Testing basic-web-app..."

    # Create a temporary test pod
    kubectl --kubeconfig="$KUBECONFIG_PATH" run temp-test --image=nginx:alpine --rm -i --restart=Never -- \
        curl -s -H "Host: myapp.local" "http://$CLUSTER_IP" | grep -q "Basic Web App" && \
        log "✅ basic-web-app test passed" || \
        error "❌ basic-web-app test failed"
}

test_multi_service_routing() {
    log "Testing multi-service-routing..."

    local test_pod="temp-test-$$"

    # Test different paths
    kubectl --kubeconfig="$KUBECONFIG_PATH" run "$test_pod" --image=nginx:alpine --rm -i --restart=Never -- sh -c "
        curl -s -H 'Host: platform.local' 'http://$CLUSTER_IP/app' | grep -q 'Frontend Application' && echo 'app: ✅' || echo 'app: ❌'
        curl -s -H 'Host: platform.local' 'http://$CLUSTER_IP/api' | grep -q 'API Backend' && echo 'api: ✅' || echo 'api: ❌'
        curl -s -H 'Host: platform.local' 'http://$CLUSTER_IP/admin' | grep -q 'Admin Interface' && echo 'admin: ✅' || echo 'admin: ❌'
    "
}

test_virtual_hosts() {
    log "Testing virtual-hosts..."

    local test_pod="temp-test-$$"

    # Test different hosts
    kubectl --kubeconfig="$KUBECONFIG_PATH" run "$test_pod" --image=nginx:alpine --rm -i --restart=Never -- sh -c "
        curl -s -H 'Host: webapp.local' 'http://$CLUSTER_IP' | grep -q 'Main Web Application' && echo 'webapp: ✅' || echo 'webapp: ❌'
        curl -s -H 'Host: api.local' 'http://$CLUSTER_IP' | grep -q 'API Service' && echo 'api: ✅' || echo 'api: ❌'
        curl -s -H 'Host: blog.local' 'http://$CLUSTER_IP' | grep -q 'Company Blog' && echo 'blog: ✅' || echo 'blog: ❌'
    "
}

# Clean up an example
cleanup_example() {
    local example="$1"
    local example_file="$example.yaml"

    if [[ ! -f "$example_file" ]]; then
        error "Example file '$example_file' not found"
    fi

    log "Cleaning up $example..."
    kubectl --kubeconfig="$KUBECONFIG_PATH" delete -f "$example_file" --ignore-not-found=true
    log "✅ $example cleaned up"
}

# Show usage
show_usage() {
    cat << EOF
NGINX Ingress Examples Deployment Script

Usage: $0 <command> [example-name] [cluster-name]

Commands:
  deploy <example>     Deploy an example
  test <example>       Test an example
  cleanup <example>    Clean up an example
  list                 List available examples
  help                 Show this help

Examples:
  basic-web-app        Simple single-service web app
  multi-service-routing Path-based routing example
  virtual-hosts        Host-based routing example

Default cluster: test-cluster

Examples:
  $0 deploy basic-web-app
  $0 test basic-web-app test-cluster
  $0 cleanup basic-web-app
  $0 list
EOF
}

# List available examples
list_examples() {
    log "Available examples:"
    echo "  • basic-web-app        - Simple single-service web application"
    echo "  • multi-service-routing - Path-based routing to multiple services"
    echo "  • virtual-hosts        - Host-based routing with multiple domains"
}

# Main script logic
main() {
    local command="${1:-help}"

    case "$command" in
        "deploy")
            if [[ $# -lt 2 ]]; then
                error "Usage: $0 deploy <example-name> [cluster-name]"
            fi
            check_prerequisites
            check_nginx_ingress
            deploy_example "$2"
            ;;
        "test")
            if [[ $# -lt 2 ]]; then
                error "Usage: $0 test <example-name> [cluster-name]"
            fi
            check_prerequisites
            test_example "$2"
            ;;
        "cleanup")
            if [[ $# -lt 2 ]]; then
                error "Usage: $0 cleanup <example-name> [cluster-name]"
            fi
            check_prerequisites
            cleanup_example "$2"
            ;;
        "list")
            list_examples
            ;;
        "help"|*)
            show_usage
            ;;
    esac
}

# Change to script directory
cd "$(dirname "${BASH_SOURCE[0]}")"

# Run main function
main "$@"