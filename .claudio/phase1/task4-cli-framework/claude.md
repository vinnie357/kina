# Task: CLI Framework with KIND Command Compatibility

You are working on a user interface task within Phase 1 of the kina implementation. This task focuses on implementing CLI structure maintaining complete compatibility with KIND command patterns for seamless user adoption.

## Task Objective:
Implement CLI structure maintaining compatibility with KIND command patterns, ensuring users can adopt kina as a drop-in replacement for kind with familiar command syntax and behavior.

## Task Requirements:
- CLI accepts KIND-compatible command structure
- Help system displays comprehensive usage information
- Command parsing handles complex argument combinations
- Version and build information accessible

## Dependencies:
- Task 1 (Rust Project Structure) - Module foundation required

## Deliverables:
- Complete CLI command structure with clap-based parsing
- Comprehensive help system with examples
- Configuration file support (YAML compatibility with KIND)
- Command-line override capabilities

## Context Integration:
- Phase Context: ../tasks.md
- Related Tasks: task1-rust-project-structure (foundation), task6-config-schema (parallel)
- Shared Resources: ../../shared/

## Implementation Guidelines:
**KIND Command Structure Compatibility**:
```bash
kina create cluster [flags]      # Mirrors: kind create cluster
kina delete cluster [flags]      # Mirrors: kind delete cluster
kina get clusters               # Mirrors: kind get clusters
kina get nodes [flags]          # Mirrors: kind get nodes
kina load container-image [flags] # Mirrors: kind load docker-image
kina export kubeconfig [flags]  # Mirrors: kind export kubeconfig
kina build node-image [flags]   # Mirrors: kind build node-image
```

**CLI Framework Requirements**:
- Use `clap` for command-line argument parsing
- Maintain exact flag compatibility with KIND where applicable
- Provide clear help text with examples
- Support global flags (--verbose, --config, etc.)
- Handle configuration file merging with command-line overrides

**User Experience Enhancements**:
- Rich error messages with suggestions
- Progress indicators for long-running operations
- Colored output for better readability
- Tab completion support preparation

**Apple Container Adaptations**:
- Update help text to reflect Apple Container usage
- Modify image-related commands for Apple Container patterns
- Adjust networking-related flags based on VM architecture

## Success Criteria:
- All major KIND commands have equivalent kina implementations
- Command parsing handles edge cases and complex flag combinations
- Help system provides clear guidance for users migrating from KIND
- Configuration system supports both CLI and file-based configuration
- Error handling provides helpful user guidance

## Next Steps:
After completing this task:
1. Update status.md with command structure and compatibility matrix
2. Coordinate with task6-config-schema for configuration integration
3. Provide CLI foundation for Phase 2 command implementation