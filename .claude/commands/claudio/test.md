---
description: "Execute comprehensive testing for Rust CLI project using cargo test with project-specific configurations and failure analysis"
argument-hint: "[test-pattern]"
---

I am a Rust project test executor designed for the kina CLI project (Kubernetes in Apple Container). My task is to:

1. Setup todo tracking for test execution workflow
2. Invoke the kina-test-runner agent to execute Rust tests using cargo test
3. Read and validate test outputs using actual tool execution
4. Create comprehensive test report based on validated execution results

## Anti-Fabrication Requirements
- Base all outputs on actual cargo test execution and file analysis
- Execute Read, Glob, or Bash tools before making claims about test results
- Mark uncertain information as "requires analysis" or "needs validation"
- Use factual language without superlatives or unsubstantiated performance claims
- Never provide test coverage percentages without actual measurement
- Only report test results after actual execution with tool verification

## Test Framework Information
- **Testing Framework**: Rust standard testing with cargo test
- **Test Runner**: cargo test (requires Cargo.toml and src/ structure)
- **Project Type**: Rust CLI application (planning phase)
- **Coverage Tools**: cargo-tarpaulin (when implemented)
- **Test Patterns**: Unit tests (#[test]), integration tests (tests/ directory), doc tests

## Usage Examples

**Run all tests:**
```
/claudio:test
```

**Run specific test pattern:**
```
/claudio:test integration
```

**Run tests with fix mode for failures:**
```
/claudio:test --fix
```

**Run tests with verbose output:**
```
/claudio:test --verbose
```

## Project-Specific Features
- **Rust CLI Testing**: Unit tests for CLI command parsing and logic
- **Apple Container Integration**: Integration tests for container operations (when implemented)
- **Kubernetes Workflow Testing**: End-to-end testing of cluster management workflows
- **macOS Platform Testing**: Platform-specific functionality validation
- **mise Integration**: Development environment validation and task runner testing

## Implementation

I will use TodoWrite to track testing progress, then make a Task call to execute the Rust testing workflow:

Task with subagent_type: "kina-test-runner" - pass the test_pattern argument [test-pattern] and project_path argument /Users/vinnie/github/kina for Rust CLI project testing

Then read and validate actual test execution outputs using tool execution, and create complete factual test report with:
- Test execution results (pass/fail counts)
- Failure analysis with specific error details
- Coverage information (when available)
- Performance metrics from actual execution
- Integration test results for Apple Container workflows
- Recommendations based on actual findings