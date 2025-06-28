# Task Completion Checklist

When completing any coding task in this project, ensure the following:

## Before Committing Code

### 1. Code Formatting
```bash
cargo fmt
```

### 2. Linting
```bash
cargo clippy --all-targets --all-features
```

### 3. Tests
```bash
cargo test
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

## Code Review Checklist
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Tests pass
- [ ] Code follows project style
- [ ] Error handling is graceful
- [ ] No hardcoded values (use constants.rs)
- [ ] Documentation updated if needed
- [ ] Binary size still under 5MB (for release builds)

## Performance Validation
- Profile if performance-critical code was changed
- Verify no unnecessary allocations introduced
- Check that streaming/polling intervals are respected