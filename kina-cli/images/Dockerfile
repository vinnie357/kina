# Kina Kubernetes Node Image
# Custom Debian-based image for Apple Container VMs with full kernel support
# Replaces kindest/node with proper VM-compatible image for Cilium CNI

FROM debian:13-slim

ARG CNI_PLUGINS_VERSION=1.9.1
ARG CONTAINERD_VERSION=2.3.1
ARG KUBERNETES_VERSION=1.35.5
ARG RUNC_VERSION=1.4.2

# Metadata
LABEL description="Kina Kubernetes Node Image for Apple Container VMs"
LABEL maintainer="kina"
LABEL version="${KUBERNETES_VERSION}"

ENV CNI_PLUGINS_VERSION=${CNI_PLUGINS_VERSION}
ENV CONTAINERD_VERSION=${CONTAINERD_VERSION}
ENV DEBIAN_FRONTEND=noninteractive
ENV KUBERNETES_VERSION=${KUBERNETES_VERSION}
ENV RUNC_VERSION=${RUNC_VERSION}

# Install base system packages (Apple Container VM provides kernel)
RUN apt-get update && apt-get install -y \
    # Base system \
    ca-certificates \
    curl \
    gnupg \
    init \
    systemd \
    systemd-sysv \
    # Network utilities \
    iproute2 \
    ipset \
    iptables \
    netcat-openbsd \
    socat \
    # Module loading utilities (for Apple Container VM kernel) \
    kmod \
    # Container runtime dependencies \
    erofs-utils \
    fuse-overlayfs \
    # Utilities \
    htop \
    jq \
    nano \
    vim \
    wget \
    # Cleanup \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Configure systemd for container environment (following Kind patterns)
ENV container=docker
STOPSIGNAL SIGRTMIN+3

RUN systemctl set-default multi-user.target \
    && systemctl mask \
         console-getty.service \
         dev-hugepages.mount \
         sys-fs-fuse-connections.mount \
         systemd-logind.service \
         systemd-tmpfiles-setup.service \
         systemd-update-utmp.service \
    && systemctl disable systemd-resolved \
    && systemctl disable systemd-timesyncd

# Install containerd
RUN curl -fsSL https://github.com/containerd/containerd/releases/download/v${CONTAINERD_VERSION}/containerd-${CONTAINERD_VERSION}-linux-arm64.tar.gz \
    | tar -xzC /usr/local

# Install runc
RUN curl -fsSL https://github.com/opencontainers/runc/releases/download/v${RUNC_VERSION}/runc.arm64 \
    -o /usr/local/sbin/runc \
    && chmod +x /usr/local/sbin/runc

# Install CNI plugins
RUN mkdir -p /opt/cni/bin \
    && curl -fsSL https://github.com/containernetworking/plugins/releases/download/v${CNI_PLUGINS_VERSION}/cni-plugins-linux-arm64-v${CNI_PLUGINS_VERSION}.tgz \
    | tar -xzC /opt/cni/bin

# Configure containerd
RUN mkdir -p /etc/containerd \
    && containerd config default > /etc/containerd/config.toml \
    && sed -i 's/SystemdCgroup = false/SystemdCgroup = true/' /etc/containerd/config.toml

# Create containerd systemd service
RUN cat > /etc/systemd/system/containerd.service << 'EOF'
[Unit]
Description=containerd container runtime
Documentation=https://containerd.io
After=network.target local-fs.target

[Service]
ExecStartPre=-/sbin/modprobe overlay
ExecStart=/usr/local/bin/containerd

Type=notify
Delegate=yes
KillMode=process
Restart=always
RestartSec=5
LimitNPROC=infinity
LimitCORE=infinity
LimitNOFILE=infinity
TasksMax=infinity
OOMScoreAdjust=-999

[Install]
WantedBy=multi-user.target
EOF

# Add Kubernetes apt repository (version derived from ARG for reproducible pinning)
RUN KUBE_MINOR="v$(echo "${KUBERNETES_VERSION}" | cut -d. -f1,2)" \
    && curl -fsSL "https://pkgs.k8s.io/core:/stable:/${KUBE_MINOR}/deb/Release.key" \
    | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/${KUBE_MINOR}/deb/ /" \
    > /etc/apt/sources.list.d/kubernetes.list

# Install Kubernetes components
RUN apt-get update && apt-get install -y \
    kubelet=${KUBERNETES_VERSION}-1.1 \
    kubeadm=${KUBERNETES_VERSION}-1.1 \
    kubectl=${KUBERNETES_VERSION}-1.1 \
    && apt-mark hold kubelet kubeadm kubectl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Configure kubelet for systemd cgroup driver
RUN mkdir -p /var/lib/kubelet \
    && cat > /var/lib/kubelet/config.yaml << 'EOF'
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
containerRuntimeEndpoint: unix:///var/run/containerd/containerd.sock
EOF

# Create Kind-compatible directory structure for kina CLI compatibility
RUN mkdir -p /kind

# Enable services
RUN systemctl enable containerd kubelet

# Configure kernel modules for Cilium
RUN cat > /etc/modules-load.d/cilium.conf << 'EOF'
# Kernel modules required for Cilium CNI
br_netfilter
ip_tables
ip6_tables
iptable_nat
ip6table_nat
xt_MARK
xt_conntrack
xt_comment
xt_multiport
xt_recent
xt_statistic
xt_TCPMSS
xt_tcpudp
xt_u32
xt_socket
vxlan
EOF

# Configure sysctl for Kubernetes and Cilium
RUN cat > /etc/sysctl.d/99-kubernetes-cri.conf << 'EOF'
# Required for Kubernetes
net.bridge.bridge-nf-call-iptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.ipv4.ip_forward = 1

# Required for Cilium
net.core.bpf_jit_enable = 1
net.core.bpf_jit_harden = 0
kernel.unprivileged_bpf_disabled = 1
net.ipv4.conf.all.rp_filter = 0
EOF

# Create entrypoint script (following Kind patterns)
RUN cat > /usr/local/bin/entrypoint << 'EOF'
#!/bin/bash
set -o errexit
set -o nounset
set -o pipefail

# Load kernel modules required for Cilium CNI
echo "Loading kernel modules for Cilium..."
modprobe br_netfilter || echo "Warning: br_netfilter module not available"
modprobe overlay || echo "Warning: overlay module not available"
modprobe vxlan || echo "Warning: vxlan module not available"
modprobe xt_socket || echo "Warning: xt_socket module not available"

# Apply sysctl settings for Kubernetes and Cilium
echo "Applying sysctl settings..."
sysctl --system || echo "Warning: some sysctl settings failed"

# Ensure containerd socket directory exists
mkdir -p /var/run/containerd

# Kubelet requires /var/lib/kubelet to be a shared bind mount so that volume
# mounts propagate correctly between the host and pod namespaces.
# Without this, kubelet fails with "failed to bind-mount /var/lib/kubelet:
# operation not permitted" even when CAP_SYS_ADMIN is present.
mkdir -p /var/lib/kubelet
mount --bind /var/lib/kubelet /var/lib/kubelet
mount --make-shared /var/lib/kubelet

# Execute the original command (typically /sbin/init)
echo "Starting systemd..."
exec "$@"
EOF

RUN chmod +x /usr/local/bin/entrypoint

# Expose necessary ports
EXPOSE 2379-2380 6443 10250 10259 10257

# Set up proper init system for VM (following Kind patterns)
ENTRYPOINT ["/usr/local/bin/entrypoint", "/sbin/init"]

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD systemctl is-active kubelet || exit 1
