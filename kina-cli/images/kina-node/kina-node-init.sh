#!/bin/bash
# kina-node initialization script
# Configures the VM for Kubernetes operation with Apple Container DNS integration

set -euo pipefail

# Log function
log() {
    echo "[kina-node-init] $*" | tee -a /var/log/kina/init.log
}

log "Starting kina-node initialization..."

# Configure hostname using Apple Container DNS
if [[ -n "${HOSTNAME:-}" ]]; then
    log "Setting hostname to: $HOSTNAME"
    hostnamectl set-hostname "$HOSTNAME"
    echo "127.0.0.1 $HOSTNAME" >> /etc/hosts
fi

# Configure container runtime
log "Configuring containerd for Kubernetes..."

# Ensure containerd is running
systemctl start containerd || log "Warning: Failed to start containerd"
systemctl is-active --quiet containerd && log "containerd is running" || log "Warning: containerd is not running"

# Pre-pull pause image for Kubernetes
log "Pre-pulling Kubernetes pause image..."
crictl pull registry.k8s.io/pause:3.9 || log "Warning: Failed to pull pause image"

# Configure kubelet for VM environment
log "Configuring kubelet for VM environment..."

# Create kubelet configuration directory
mkdir -p /var/lib/kubelet

# Set up basic kubelet configuration
cat > /var/lib/kubelet/config.yaml << EOF
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
containerRuntimeEndpoint: unix:///var/run/containerd/containerd.sock
cgroupDriver: systemd
failSwapOn: false
serverTLSBootstrap: true
authentication:
  anonymous:
    enabled: false
  webhook:
    enabled: true
authorization:
  mode: Webhook
clusterDomain: cluster.local
clusterDNS:
  - 10.96.0.10
EOF

# Configure kubelet service
mkdir -p /etc/systemd/system/kubelet.service.d
cat > /etc/systemd/system/kubelet.service.d/10-kubeadm.conf << EOF
[Service]
Environment="KUBELET_KUBECONFIG_ARGS=--bootstrap-kubeconfig=/etc/kubernetes/bootstrap-kubelet.conf --kubeconfig=/etc/kubernetes/kubelet.conf"
Environment="KUBELET_CONFIG_ARGS=--config=/var/lib/kubelet/config.yaml"
Environment="KUBELET_KUBEADM_ARGS=--container-runtime-endpoint=unix:///var/run/containerd/containerd.sock"
Environment="KUBELET_EXTRA_ARGS=--node-ip=\$(hostname -I | awk '{print \$1}')"
ExecStart=
ExecStart=/usr/bin/kubelet \$KUBELET_KUBECONFIG_ARGS \$KUBELET_CONFIG_ARGS \$KUBELET_KUBEADM_ARGS \$KUBELET_EXTRA_ARGS
EOF

# Configure for Apple Container VM networking
log "Configuring networking for Apple Container VM..."

# Enable IP forwarding for Kubernetes
echo 'net.ipv4.ip_forward = 1' >> /etc/sysctl.conf
echo 'net.bridge.bridge-nf-call-iptables = 1' >> /etc/sysctl.conf
echo 'net.bridge.bridge-nf-call-ip6tables = 1' >> /etc/sysctl.conf
sysctl --system || log "Warning: Failed to apply sysctl settings"

# Load kernel modules required for Kubernetes
modprobe overlay || log "Warning: Failed to load overlay module"
modprobe br_netfilter || log "Warning: Failed to load br_netfilter module"

# Add modules to load on boot
cat > /etc/modules-load.d/kina.conf << EOF
overlay
br_netfilter
EOF

# Disable swap (required by Kubernetes)
swapoff -a || log "Warning: Failed to disable swap"
sed -i '/swap/d' /etc/fstab || log "Warning: Failed to remove swap from fstab"

# Create kina metadata file
cat > /etc/kina/node-metadata.json << EOF
{
    "node_type": "${KINA_NODE_TYPE:-unknown}",
    "cluster_name": "${CLUSTER_NAME:-unknown}",
    "node_role": "${NODE_ROLE:-worker}",
    "initialized_at": "$(date -Iseconds)",
    "container_runtime": "containerd",
    "kubernetes_version": "$(kubelet --version | cut -d' ' -f2)"
}
EOF

# Set up log rotation
cat > /etc/logrotate.d/kina << EOF
/var/log/kina/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 0644 root root
}
EOF

log "kina-node initialization completed successfully"

# Signal that initialization is complete
touch /var/lib/kina/node-ready

log "Node ready for Kubernetes cluster join"