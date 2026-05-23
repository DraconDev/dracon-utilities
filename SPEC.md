# Project Spec

## Invariants

### 1. Project compiles
The Rust project must compile without errors.
```
cargo check --quiet
```

### 2. No blocking TODO comments
No TODO comments with "FIXME" or "BLOCKING" prefix in source files.
```
! grep -r "FIXME:\|BLOCKING:" src/ --include="*.rs" 2>/dev/null
```

### 3. Tests pass
Unit tests must pass.
```
cargo test --quiet
```
