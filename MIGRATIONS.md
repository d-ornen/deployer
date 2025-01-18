# Migrations guide

## From `<=1.2.1` to `1.3.0`

1. Edit `deploy-config.json` and rename `inplace_artifacts_into_project_root` to `place_artifacts_into_project_root`.
2. Remove `tags` field inside your deploy-like and `Observe` Actions.
