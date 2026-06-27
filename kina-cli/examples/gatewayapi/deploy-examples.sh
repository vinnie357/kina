#!/bin/bash

# Gateway API Examples Deployment Script for Kina
#
# Usage: ./deploy-examples.sh <command> [example-name] [cluster-name]
#
# Commands:
#   deploy <name|all>    Deploy an example (substitutes NODE_IP automatically)
#   test <name>          Quick smoke-test an HTTP example
#   cleanup <name|all>   Remove example resources
#   list                 List available examples
#   help                 Show this help

set -euo pipefail

CLUSTER_NAME="${2:-arena}"
KUBECONFIG_PATH="$HOME/.kube/$CLUSTER_NAME"
NODE_IP=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

HTTP_EXAMPLES=(http-basic http-virtual-hosts cross-namespace-referencegrant)
ALL_EXAMPLES=(http-basic http-virtual-hosts tcp-route udp-route cross-namespace-referencegrant)

check_prerequisites() {
    log "Checking prerequisites..."

    command -v kubectl &>/dev/null || error "kubectl not found"
    command -v kina    &>/dev/null || error "kina not found"

    [[ -f "$KUBECONFIG_PATH" ]] || \
        error "Kubeconfig not found at $KUBECONFIG_PATH. Create with: kina create $CLUSTER_NAME"

    # Detect node IP from the cluster
    NODE_IP=$(kubectl --kubeconfig="$KUBECONFIG_PATH" get nodes \
        -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}' 2>/dev/null || true)
    [[ -n "$NODE_IP" ]] || error "Could not determine node IP from cluster $CLUSTER_NAME"

    log "Node IP: $NODE_IP"

    # Verify NGF is present
    kubectl --kubeconfig="$KUBECONFIG_PATH" get ns nginx-gateway &>/dev/null || \
        error "nginx-gateway namespace not found. Run: kina install nginx-gateway-fabric --cluster $CLUSTER_NAME"
}

apply_example() {
    local name="$1"
    local file="${name}.yaml"

    [[ -f "$file" ]] || error "Example file not found: $file"

    log "Deploying $name (node IP: $NODE_IP)..."
    sed "s/<NODE_IP>/$NODE_IP/g" "$file" | kubectl --kubeconfig="$KUBECONFIG_PATH" apply -f -
    log "$name deployed."
}

test_http_example() {
    local name="$1"
    local host=""

    case "$name" in
        http-basic)             host="hello.$NODE_IP.nip.io" ;;
        http-virtual-hosts)     host="app-a.$NODE_IP.nip.io" ;;
        cross-namespace-referencegrant) host="cross-ns.$NODE_IP.nip.io" ;;
        *) warn "No HTTP test defined for $name — skipping"; return ;;
    esac

    log "Testing http://$host ..."
    local code
    code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "http://$host" || true)
    if [[ "$code" == "200" ]]; then
        log "$name test passed (HTTP 200)"
    else
        warn "$name returned HTTP $code (expected 200) — check NGF logs"
    fi
}

cleanup_example() {
    local name="$1"
    local file="${name}.yaml"

    [[ -f "$file" ]] || error "Example file not found: $file"

    log "Cleaning up $name..."
    sed "s/<NODE_IP>/0.0.0.0/g" "$file" | \
        kubectl --kubeconfig="$KUBECONFIG_PATH" delete -f - --ignore-not-found=true
    log "$name cleaned up."
}

list_examples() {
    echo "HTTP examples (no Gateway patches needed beyond HTTP listener):"
    echo "  http-basic                     Single Deployment + HTTPRoute"
    echo "  http-virtual-hosts             Two apps on different nip.io hostnames"
    echo "  cross-namespace-referencegrant HTTPRoute in 'routing' ns, Service in 'backend' ns"
    echo ""
    echo "L4 examples (require experimental CRDs + kina-43 RBAC + tcp/udp listeners):"
    echo "  tcp-route                      TCP echo server via TCPRoute"
    echo "  udp-route                      UDP echo server via UDPRoute"
    echo ""
    echo "See README.md for the full prerequisite setup."
}

show_usage() {
cat <<EOF
Gateway API Examples for Kina

Usage: $0 <command> [example-name|all] [cluster-name]

Commands:
  deploy <name|all>    Deploy example(s); auto-substitutes NODE_IP
  test <name>          HTTP smoke-test (HTTP examples only)
  cleanup <name|all>   Delete example resources
  list                 List available examples
  help                 Show this help

Default cluster: arena

Examples:
  $0 deploy http-basic
  $0 deploy all
  $0 test http-basic
  $0 cleanup http-basic arena
EOF
}

main() {
    local cmd="${1:-help}"
    cd "$(dirname "${BASH_SOURCE[0]}")"

    case "$cmd" in
        deploy)
            [[ $# -ge 2 ]] || error "Usage: $0 deploy <name|all> [cluster]"
            check_prerequisites
            if [[ "$2" == "all" ]]; then
                for ex in "${HTTP_EXAMPLES[@]}"; do apply_example "$ex"; done
            else
                apply_example "$2"
            fi
            ;;
        test)
            [[ $# -ge 2 ]] || error "Usage: $0 test <name> [cluster]"
            check_prerequisites
            test_http_example "$2"
            ;;
        cleanup)
            [[ $# -ge 2 ]] || error "Usage: $0 cleanup <name|all> [cluster]"
            check_prerequisites
            if [[ "$2" == "all" ]]; then
                for ex in "${ALL_EXAMPLES[@]}"; do cleanup_example "$ex"; done
            else
                cleanup_example "$2"
            fi
            ;;
        list)   list_examples ;;
        help|*) show_usage ;;
    esac
}

main "$@"
