# Arcane Next Phase

## Strategy - The "Kill Coolify" Plan

-   [x] Compare Features & Define Scope (docs/COOLIFY_ANALYSIS.md)
-   [ ] **Phase 1: Environment Management** (Unlocks Staging/Prod)
    -   [ ] Create `config/envs/` structure code
    -   [ ] Update `servers.toml` to support environments
    -   [ ] Encrypt/Decrypt logic for env-specific keys
-   [ ] **Phase 2: Docker Compose Support** (Unlocks Chimera/Citadel)
    -   [ ] Auto-detect `docker-compose.yaml`
    -   [ ] Implement `arcane deploy` logic for Compose (build, push, up)
    -   [ ] Handle persistent volumes (ensure data survives)
-   [ ] **Phase 3: Server Groups** (Quality of Life)
    -   [ ] Define groups in `servers.toml`
    -   [ ] Implement parallel deploy logic
-   [ ] **Phase 4: Remote Logs** (Observability)
    -   [ ] Implement `arcane logs <server> <container>` via SSH

## Notes

-   Chimera uses gRPC and multiple ports -> Caddy handles this better than Traefik.
-   "Arcane Spark" (build server) is deferred. The security model is ready when we need it.
