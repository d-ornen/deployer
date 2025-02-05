# Migrations guide

## From `<=1.3.5` to `1.4.0`

Migrations will be almost fully applied automatically on configuration save. Deployer will be able to work with old configuration formats (`"version": 2`).

1. All Actions will be typed internally with `type` field and `snake_case` (example: `{ "type": "build", ... }`).
2. `deploy-config.json` and `deploy-global.json` will be upgraded to `"version": 3`.
3. Variables, requirements, placements and other structs are changed, see the documentation.
4. Check Action will become Test, original Test will be migrated into PostBuild.

### Need manual changes:

1. Cannot automatically migrate targets and programming languages; fill them by yourself.

## From `<=1.3.4` to `1.3.5`

1. Use `deployer run` instead of `deployer build`. Also use `deployer run --run-at` instead of `--build-at`.
2. Rename inner `BuildEnvironment` to `RunEnvironment` and replace `build_dir` with `run_dir`.

## From `<=1.2.1` to `1.3.0`

1. Edit `deploy-config.json` and rename `inplace_artifacts_into_project_root` to `place_artifacts_into_project_root`.
2. Remove `tags` field inside your deploy-like and `Observe` Actions.
3. `deploy-config.json` and `deploy-global.json` will be upgraded to `"version": 2`.
