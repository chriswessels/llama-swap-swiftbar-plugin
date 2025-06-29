# Task Completion Checklist

When completing any coding task in this project, ensure the following:

## Before Committing Code

### 1. Code Formatting
```bash
cargo fmt
```

### 2. Linting (Zero Warnings Required)
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Tests
```bash
# Run all tests
cargo test

# Run specific test files
cargo test --test metrics_tests
cargo test --test install_ux_tests
cargo test --test sleep_mechanism_tests
```

### 4. Build Verification
```bash
# Verify debug build
cargo build

# Verify release build  
cargo build --release
```

### 5. Manual Testing (if applicable)
- Test the specific feature/fix manually
- Verify no regressions in related functionality
- Check memory usage remains stable
- Ensure CPU usage is minimal
- Test service management operations
- Verify state transitions work correctly

## Code Review Checklist
- [ ] No compiler warnings
- [ ] No clippy warnings (enforced with -D warnings)
- [ ] All tests pass
- [ ] Code follows project style
- [ ] Error handling is graceful using Result<T> and error_helpers
- [ ] No hardcoded values (use constants.rs)
- [ ] State transitions are logged with eprintln!
- [ ] Documentation updated if needed
- [ ] Binary size still under 5MB (for release builds)
- [ ] Tests added to appropriate test file in tests/ directory

## Performance Validation
- Profile if performance-critical code was changed
- Verify no unnecessary allocations introduced in streaming loop
- Check that polling intervals are respected
- Ensure System objects are reused for sysinfo calls
- Verify historical metrics are preserved across failures

## State Management Validation
- Verify state transitions are correct
- Check that service status is properly tracked
- Ensure polling mode adapts to activity levels
- Validate error recovery preserves metrics history
- Test installation UX for missing components

## Testing Strategy
- Unit tests for individual functions (metrics collection, state transitions)
- Integration tests for user experience (install flow, menu generation)
- System tests for service management operations
- Error condition tests for API failures and recovery