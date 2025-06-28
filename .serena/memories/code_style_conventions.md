# Code Style and Conventions

## Rust Style Guidelines
- Follow Rust standard style (enforced by rustfmt)
- Use descriptive variable names
- Add doc comments for public APIs
- Keep functions focused and small
- Use type aliases for clarity (e.g., `type Result<T> = std::result::Result<T, Box<dyn Error>>`)

## Error Handling Pattern
```rust
// Use Result type for fallible operations with context
pub fn fetch_metrics() -> Result<Metrics> {
    let response = client.get(url)
        .send()
        .map_err(|e| format!("Failed to connect: {}", e))?;
    
    let mut metrics: Metrics = response.json()?;
    metrics.validate()?;
    
    Ok(metrics)
}

// Handle errors gracefully in UI
match fetch_metrics() {
    Ok(metrics) => display_metrics(metrics),
    Err(e) => {
        eprintln!("Metrics error: {}", e);
        display_offline_state()
    }
}
```

## Module Organization
- Each module has a single responsibility
- Public API at the top of files
- Implementation details below
- Constants in dedicated constants.rs file

## Performance Considerations
- Minimize allocations in hot paths
- Reuse buffers where possible
- Profile before optimizing
- Keep binary size under 5MB (uses aggressive optimization in release)

## Naming Conventions
- Snake_case for functions and variables
- PascalCase for types and traits
- SCREAMING_SNAKE_CASE for constants
- Descriptive names over abbreviations