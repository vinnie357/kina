---
description: "AI-powered test analysis and optimization for Rust CLI project using Gemini integration for intelligent failure diagnosis and solution generation"
argument-hint: "[analysis-mode]"
---

I am an AI-powered test analysis system designed for the kina Rust CLI project (Kubernetes in Apple Container). My task is to:

1. Setup todo tracking for AI-powered test analysis workflow
2. Execute cargo test and capture comprehensive test output
3. Use Gemini AI integration for intelligent analysis of test results
4. Generate AI-powered recommendations and fixes for test failures
5. Create comprehensive analysis report with AI insights

## Requirements
- **gemini-cli**: Install with `brew install gemini-cli` or `npm install -g @google/gemini-cli`
- **Gemini API Access**: Configure with `gemini config set api-key YOUR_API_KEY`
- **Rust Development**: Requires cargo and rustc for test execution
- **Project Structure**: Works with Cargo.toml and standard Rust test structure

## Anti-Fabrication Requirements
- Base all outputs on actual cargo test execution and Gemini AI analysis
- Execute Bash tools to run cargo test before making claims about results
- Use Gemini AI only for analysis of actual test output, not fabricated scenarios
- Mark AI predictions as "AI analysis suggests" or "requires validation"
- Never fabricate test results or AI responses without actual tool execution

## Usage Examples

**AI analysis of all test failures:**
```
/claudio:test-g
```

**AI-powered analysis of specific test pattern:**
```
/claudio:test-g integration
```

**Generate AI fixes for failing tests:**
```
/claudio:test-g --fix
```

**Comprehensive AI test analysis with optimization suggestions:**
```
/claudio:test-g --optimize
```

## AI Analysis Features
- **Failure Pattern Recognition**: AI analysis of common Rust test failure patterns
- **Solution Generation**: AI-powered fix recommendations for test failures
- **Performance Analysis**: AI insights on test performance bottlenecks
- **Code Quality Assessment**: AI evaluation of test code quality and coverage
- **Architecture Recommendations**: AI suggestions for test structure improvements
- **Rust-Specific Analysis**: AI expertise in Rust testing best practices and idioms

## Implementation

I will execute the AI-powered testing workflow using direct Bash tool integration with gemini-cli:

```bash
# Execute cargo test and capture output
cargo test --verbose 2>&1 | tee test_output.log

# Use Gemini AI for intelligent analysis
gemini -y -p "Analyze this Rust cargo test output for the kina CLI project (Kubernetes in Apple Container).

Project Context:
- Rust CLI application for Kubernetes cluster management using Apple Container
- Target platform: macOS with Apple Container integration
- Development stage: Planning phase, may not have full implementation yet
- Key dependencies: Apple Container CLI, Kubernetes tools, mise for development

Test Output Analysis Request:
$(cat test_output.log)

Please provide:
1. **Test Execution Summary**: Pass/fail counts, execution time, any setup issues
2. **Failure Analysis**: Detailed analysis of any test failures with root cause identification
3. **Rust-Specific Issues**: Analysis of Rust-specific problems (compilation errors, dependency issues, etc.)
4. **CLI Testing Recommendations**: Suggestions for testing CLI command parsing and workflow logic
5. **Integration Testing Strategy**: Recommendations for Apple Container and Kubernetes integration testing
6. **Performance Insights**: Analysis of test performance and optimization opportunities
7. **Next Steps**: Prioritized action items based on current project state

Focus on practical, actionable recommendations for improving the Rust CLI testing approach.

--- END GEMINI ANALYSIS ---"
```

The AI analysis will provide intelligent insights into:
- **Test failure patterns** specific to Rust CLI applications
- **Apple Container integration** testing strategies
- **Kubernetes workflow** testing approaches
- **Platform-specific issues** for macOS development
- **Development environment** optimization with mise integration
- **Code quality improvements** based on test results analysis