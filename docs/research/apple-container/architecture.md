# Apple Container Technical Architecture

**Focus**: Detailed technical analysis of Apple's Containerization framework and Container CLI tool

## Framework Components

### Apple Containerization Framework (Swift Package)
- **Package Name**: `Containerization`
- **Primary Language**: Swift
- **Integration**: Native macOS Virtualization.framework
- **Target Platform**: Apple Silicon Macs with macOS 26+
- **Repository**: https://github.com/apple/container

### Container CLI Tool Architecture
- **Implementation**: Swift-based command-line interface
- **Version**: v0.1.0 (early development stage)
- **Architecture**: Lightweight VM management with OCI image support

## VM-per-Container Architecture

Apple's approach differs fundamentally from traditional container runtimes:

```
Traditional Docker/containerd:
Host OS → Docker Engine → Shared Linux VM → Multiple Containers (namespace isolation)

Apple Container:
Host macOS → Containerization Framework → Individual Linux VMs → Single Container per VM
```

### Technical Implementation Details

**Virtual Machine Characteristics:**
- Each container executes in a dedicated lightweight virtual machine
- Leverages macOS Virtualization.framework for hardware acceleration
- Custom init system `vminitd` written in Swift runs as PID 1 in each VM
- Sub-second container startup times through kernel optimization
- Dedicated IP address per container eliminates port forwarding complexity

**Virtualization.framework Integration:**
- Hardware-accelerated virtualization on Apple Silicon
- VM resource allocation and management
- Memory isolation and security boundaries
- Network stack virtualization per container

## API Surface and Swift Integration

### Core Swift APIs (from Containerization package)
- Container lifecycle management (spawn, manage, interact)
- OCI image operations (pull, push, inspect)
- Ext4 filesystem creation and management
- Netlink socket family interactions
- Virtualization.framework integration
- gRPC API for init system communication

### Container CLI Commands (Complete Output)

```bash
# Full command help output from `container --help`:
OVERVIEW: A container platform for macOS

USAGE: container [--debug] <subcommand>

OPTIONS:
  --debug                 Enable debug output [environment: CONTAINER_DEBUG]
  --version               Show the version.
  -h, --help              Show help information.

CONTAINER SUBCOMMANDS:
  create                  Create a new container
  delete, rm              Delete one or more containers
  exec                    Run a new command in a running container
  inspect                 Display information about one or more containers
  kill                    Kill one or more running containers
  list, ls                List containers
  logs                    Fetch container stdio or boot logs
  run                     Run a container
  start                   Start a container
  stop                    Stop one or more running containers

IMAGE SUBCOMMANDS:
  build                   Build an image from a Dockerfile
  images, image, i        Manage images
  registry, r             Manage registry configurations

VOLUME SUBCOMMANDS:
  volume, v               Manage container volumes

OTHER SUBCOMMANDS:
  builder                 Manage an image builder instance
  system, s               Manage system components
```

### Key Commands for kina Integration

```bash
# Container lifecycle operations
container create <image>        # Create new container
container run <image> [command] # Run container
container start <container>     # Start existing container
container stop <container>      # Stop running container
container delete <container>    # Delete container (alias: rm)
container list                  # List containers (alias: ls)
container inspect <container>   # Get container details
container exec <container> [command]  # Execute command in container
container logs <container>      # Get container logs

# Image management
container images               # List images (alias: image, i)
container build               # Build image from Dockerfile
container registry            # Manage registry config (alias: r)

# System and volume management
container volume              # Manage volumes (alias: v)
container system              # System management (alias: s)
```

## Container Runtime Architecture

### VM Initialization Process

1. **VM Creation**: Virtualization.framework creates lightweight Linux VM
2. **Kernel Boot**: Optimized Linux kernel boots in sub-second timeframe
3. **Init System**: Swift-based `vminitd` starts as PID 1
4. **Container Process**: Target container process launches within VM
5. **Network Setup**: Dedicated IP address assigned to VM
6. **Filesystem Mount**: Container filesystem mounted within VM environment

### Resource Management

**CPU Allocation:**
- Dedicated CPU cores or threads per VM
- Hardware-level CPU isolation
- Apple Silicon performance core utilization
- Dynamic CPU scaling capabilities

**Memory Management:**
- Dedicated memory allocation per VM
- Hardware-enforced memory boundaries
- Memory ballooning and optimization
- Efficient memory sharing where possible

**Network Architecture:**
- Individual IP addresses per container VM
- Native macOS network stack integration
- Hardware-accelerated networking
- Simplified port management (no conflicts)

**Storage System:**
- Ext4 filesystem within each VM
- Copy-on-write optimizations
- Volume mounting capabilities
- Image layer management

## System Integration Patterns

### macOS Virtualization.framework Integration

```swift
// Conceptual Swift API patterns
import Virtualization

class ContainerVM {
    private var virtualMachine: VZVirtualMachine
    private var configuration: VZVirtualMachineConfiguration

    func createContainer(config: ContainerConfig) throws -> ContainerInstance {
        // Configure VM with container specifications
        let vmConfig = VZVirtualMachineConfiguration()
        vmConfig.cpuCount = config.cpuLimit ?? 1
        vmConfig.memorySize = config.memoryLimit ?? (512 * 1024 * 1024)

        // Network configuration with dedicated IP
        let networkDevice = VZNATNetworkDeviceAttachment()
        vmConfig.networkDevices = [VZVirtioNetworkDeviceConfiguration(attachment: networkDevice)]

        // Storage configuration
        let bootLoader = VZLinuxBootLoader(kernelURL: kernelURL)
        vmConfig.bootLoader = bootLoader

        // Create and start VM
        virtualMachine = VZVirtualMachine(configuration: vmConfig)
        try await virtualMachine.start()

        return ContainerInstance(vm: virtualMachine, config: config)
    }
}
```

### gRPC Communication Architecture

The `vminitd` init system uses gRPC for communication:

```protobuf
// Conceptual gRPC service for VM init system
service VMInitService {
    rpc StartContainer(StartContainerRequest) returns (StartContainerResponse);
    rpc StopContainer(StopContainerRequest) returns (StopContainerResponse);
    rpc ExecuteCommand(ExecuteCommandRequest) returns (stream ExecuteCommandResponse);
    rpc GetContainerStatus(GetStatusRequest) returns (GetStatusResponse);
}

message StartContainerRequest {
    string image = 1;
    repeated string command = 2;
    map<string, string> environment = 3;
    repeated VolumeMount volumes = 4;
}
```

## Architecture Comparison

### Traditional Container Runtime
```
┌─────────────────────────────────────────────────────────────┐
│                        Host OS (macOS)                      │
├─────────────────────────────────────────────────────────────┤
│                    Docker Desktop                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                  Linux VM                               │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │ │
│  │  │ Container 1 │ │ Container 2 │ │ Container 3 │  ...  │ │
│  │  │ (namespace) │ │ (namespace) │ │ (namespace) │       │ │
│  │  └─────────────┘ └─────────────┘ └─────────────┘       │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Apple Container Runtime
```
┌─────────────────────────────────────────────────────────────┐
│                        Host OS (macOS)                      │
├─────────────────────────────────────────────────────────────┤
│              Containerization Framework                     │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │   Linux VM  │ │   Linux VM  │ │   Linux VM  │    ...    │
│  │  ┌─────────┐│ │  ┌─────────┐│ │  ┌─────────┐│           │
│  │  │Container││ │  │Container││ │  │Container││           │
│  │  │    1    ││ │  │    2    ││ │  │    3    ││           │
│  │  └─────────┘│ │  └─────────┘│ │  └─────────┘│           │
│  └─────────────┘ └─────────────┘ └─────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

## Performance Characteristics

### Startup Performance
- **VM Boot Time**: < 1 second with optimized kernel
- **Container Init**: Near-instantaneous after VM ready
- **Total Startup**: Sub-second from `container run` to process execution
- **Memory Overhead**: ~50MB per VM vs ~100MB traditional containers

### Resource Isolation Benefits
- **CPU Isolation**: Dedicated CPU allocation prevents resource contention
- **Memory Boundaries**: Hardware-enforced memory isolation
- **Network Stack**: Individual IP addresses eliminate port conflicts
- **Filesystem**: Dedicated filesystem per VM prevents I/O interference

### Scalability Considerations
- **VM Overhead**: Each container requires dedicated VM resources
- **Resource Management**: Efficient VM resource allocation required
- **Density Trade-offs**: Higher security isolation vs container density
- **Performance Scaling**: Linear scaling with dedicated resources per container