# kina-node: Kubernetes node image optimized for Apple Container VM architecture
# Based on Ubuntu 22.04 LTS with full container runtime and Kubernetes components

FROM ubuntu:22.04

# Avoid interactive prompts during build
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies and utilities
RUN apt-get update && apt-get install -y \
    systemd systemd-sysv \
    curl wget gnupg lsb-release \
    ca-certificates apt-transport-https \
    iptables iproute2 iputils-ping \
    openssh-server \
    socat conntrack \
    && rm -rf /var/lib/apt/lists/*

# Configure systemd for container environment
RUN systemctl enable ssh

# Install containerd container runtime
RUN curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg && \
    echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null && \
    apt-get update && apt-get install -y containerd.io && \
    rm -rf /var/lib/apt/lists/*

# Configure containerd for Kubernetes
RUN mkdir -p /etc/containerd && \
    containerd config default | tee /etc/containerd/config.toml && \
    sed -i 's/SystemdCgroup = false/SystemdCgroup = true/' /etc/containerd/config.toml && \
    systemctl enable containerd

# Add Kubernetes repository and install components
RUN curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.31/deb/Release.key | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg && \
    echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.31/deb/ /' | tee /etc/apt/sources.list.d/kubernetes.list && \
    apt-get update && apt-get install -y kubelet kubeadm kubectl && \
    apt-mark hold kubelet kubeadm kubectl && \
    systemctl enable kubelet && \
    rm -rf /var/lib/apt/lists/*

# Configure kubelet for VM environment
RUN mkdir -p /etc/kubernetes/kubelet.conf.d

# Install CNI plugins for pod networking within VM
RUN mkdir -p /opt/cni/bin && \
    curl -L "https://github.com/containernetworking/plugins/releases/download/v1.3.0/cni-plugins-linux-amd64-v1.3.0.tgz" | tar -C /opt/cni/bin -xz

# Create kina-specific directories and configurations
RUN mkdir -p /etc/kina /var/lib/kina /var/log/kina

# Copy kina-node initialization script
COPY kina-node-init.sh /usr/local/bin/kina-node-init.sh
RUN chmod +x /usr/local/bin/kina-node-init.sh

# Configure kina-node service to run on startup
RUN cat > /etc/systemd/system/kina-node-init.service << 'EOF'
[Unit]
Description=kina Node Initialization
After=containerd.service
Requires=containerd.service

[Service]
Type=oneshot
ExecStart=/usr/local/bin/kina-node-init.sh
RemainAfterExit=true

[Install]
WantedBy=multi-user.target
EOF

RUN systemctl enable kina-node-init.service

# Set environment for Apple Container DNS integration
ENV KINA_NODE_TYPE=""
ENV CLUSTER_NAME=""
ENV NODE_ROLE=""

# Expose Kubernetes API server port (for control plane nodes)
EXPOSE 6443

# Expose kubelet port
EXPOSE 10250

# Use systemd as init system (required for VM environment)
CMD ["/sbin/init"]