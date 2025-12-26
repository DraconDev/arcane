# Arcane Feature Test Stack

A simple stack to verify Arcane features.

## Components

-   **Web**: Python app (Ports 8080).
    -   `/` -> Info + Env Vars
    -   `/health` -> 200 OK
    -   `/crash` -> Exits (Tests restart)
-   **Redis**: Database (Tests multi-container)

## Verification Steps

Run these commands from the PROJECT ROOT (arcane/):

1. **Deploy Single Image (Garage Mode)**:

    ```bash
    # Deploys only the python app
    cd examples/feature-test
    # Build image manually if needed, or rely on arcane (but arcane expects Dockerfile in current dir)
    # NOTE: Run arcane from this directory
    ../../target/debug/arcane deploy --target micro1 --app feature-test
    ```

2. **Deploy Stack (Compose)**:

    ```bash
    cd examples/feature-test
    ../../target/debug/arcane deploy --target micro1 --compose docker-compose.yaml
    ```

3. **Observability**:
    ```bash
    ../../target/debug/arcane logs micro1
    ../../target/debug/arcane exec micro1 -- curl localhost:8080
    ```
