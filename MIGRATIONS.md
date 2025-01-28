# Migrations guide

## From `<=1.3.4` to `1.3.5`

1. Use `deployer run` instead of `deployer build`. Also use `deployer run --run-at` instead of `--build-at`.
2. Rename `BuildEnvironment` to `RunEnvironment` and replace `build_dir` with `run_dir`.

## From `<=1.2.1` to `1.3.0`

1. Edit `deploy-config.json` and rename `inplace_artifacts_into_project_root` to `place_artifacts_into_project_root`.
2. Remove `tags` field inside your deploy-like and `Observe` Actions.
