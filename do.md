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

- we need to expore github features more see if we have more synergies 

- one tweak we can do that in most cases we are logged into github and have the keys to the server, so can we make the setup easier? Even if big company doing it they would still need the account that can setup the webhooks right?