# Go Secret Test

A compiled Go application to verify multi-stage builds and secret injection.

## Components

-   **Go App**: Minimal HTTP server (Alpine).
    -   Port: 8080 (Mapped to 8081).
    -   Secret: `API_TOKEN` (Masked in output).

## Verification

```bash
cd examples/go-secret-test
../../target/debug/arcane deploy --target micro1 --compose docker-compose.yaml --env micro1 --dry-run
```
