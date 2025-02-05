<!-- markdownlint-disable-file MD024 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0-beta-2] - 2025-02-05

### Added

- YAML and TOML project configuration support (see documentation, top paragraph).
- Containered runs and Sub-pipeline documentation.

### Changed

- `run_strategies` to `cache_strategies` (project configuration).

### Fixed

- A lot of documentation.

## [1.4.0-beta-1] - 2025-02-05

### Added

- Test containered run support - with build/cache strategies. No documentation for now, but you can test it with `deployer run containered` *inside Deployer's project* (or manually view `containered` Pipeline).
- Content examples.
- Custom storage path support (via `DEPLOYER_STORAGE_PATH` environment variable).
- Relative cache ignore pattern support (`public/tailwind.css` or `any/other/relative/path.json`).
- Sub-pipeline support inside Actions (no documentation for now).
- R/W support for `toml` and `yaml` (no usage for now).

### Changed

- Configuration format (bumped to v4, see the `MIGRATIONS.md`).
- Removed old `Test` Action, moved `Check` to `Old`.

### Fixed

- Copy error handling (for example, crash on `text file busy os error`).

### Removed

- Python scripts support by `Patch` Actions (because of dynamic linkage and Deployer breaks after Python upgrades).

## [1.3.5] - 2025-01-28

### Changed

- `Build` -> `Run`. Including `BuildEnvironment`, `deployer build`, etc. See the `MIGRATIONS.md`.

## [1.3.4] - 2025-01-23

### Added

- Configurations' auto-migrations.

### Changed

- Bumped `smart-patcher`.

## [1.3.3] - 2025-01-21

### Added

- `ls content` sorting.
- `Observe` Action process kill on Ctrl+C.

### Changed

- Moved `AddToStorage` execution code to `storage_add.rs`.
- Disabled `Observe` Actions' I/O redirection.

### Fixed

- Auto-version cmd-based rule checkup.
- Build-like Actions setup.

## [1.3.2] - 2025-01-18

### Added

- Release optimizations (issue [#1]).
- Zero remote hosts error on `deployer build -R`.

### Fixed

- Error receiving from remote hosts.
- Content search and removing errors.
- `test` Pipeline.
- `--silent` option on `deployer build -R`.

## [1.3.1] - 2025-01-18

### Added

- `deployer docs` command.
- `tui` feature.

### Changed

- Made all structs and methods public.

## [1.3.0] - 2025-01-17

### Added

- Migration Guide and Security Policy.
- `VarValue::FromEnvVar` variable type.

### Changed

- Added and fixed module documentation.
- All setup TUI moved to `crate::tui::setup` module.

### Fixed

- Added artifacts placement just before `AddToStorage` Action.

### Removed

- `ForceArtifactsEnplace` Action.
- Tags from deploy-like and `Observe` Actions.

## [1.2.1] - 2025-01-14

### Fixed

- Remote artifacts sync.

## [1.2.0] - 2025-01-14

### Added

- `SyncToRemote` and `SyncFromRemote` Actions to sync build folders between current and remote hosts.
- Remote Action and Pipeline execution support.
- `-r` and `-R` arguments to `build` command.

### Changed

- Documentation and `i18n`.

## [1.1.0-beta-2] - 2025-01-14

### Fixed

- `replacements` and `ignore_fails` flags support on remote exec.

## [1.1.0-beta-1] - 2025-01-14

### Added

- `remote` module and `RemoteHost` entity.
- `RemoteAccessibleAndReady` requirement type.
- `remote_exec` custom command field.
- All related TUI and `i18n`.

### Changed

- Global configuration hashmaps' key types (to validate).
- Some documentation updates and fixes.

## [1.0.1] - 2025-01-14

### Fixed

- Short name validator.

## [1.0.0] - 2025-01-13

### Added

- `FromEnvFile` variable type.
- `FromHCVaultKv2` variable type (HashiCorp Vault KV2 support).
- Chunk-by-chunk file comparison on `copy_all` function calls (to discard file timestamps rewrite on equal content).
- Requirements for Actions (satisfy path, one of paths or even `Check` to run given Action).
- `TODO.md`.

### Changed

- Moved all TUI-related code in `tui` module.
- Moved output of build folder's path at the top of Pipeline.
- Made `cache_files` project config field a hashset in Deployer's internals (for reducing duplicates).
- `Cargo.toml` default features and Deployer's project config.

### Fixed

- `AutoVersionExtractFromRule` trimming.
- `README.md` and `DOCS.ru.md`.

## [0.3.4] - 2025-01-10

### Changed

- Deny current version content's removal from `AddToStorage` Actions.

## [0.3.3] - 2025-01-10

### Added

- `AddToStorage` Actions.
- `latest` version support for `UseFromStorage` Actions.

### Changed

- Rewriting content with equal infos.

### Removed

- `ProjectClean` Actions due to lack of need.

## [0.3.2] - 2025-01-08

### Added

- Support of `.deploy-config.json` (hidden) project's configurations.
- Size of files that cleaned by the `clean` command.

### Changed

- Made `smart-patcher`'s Python, Lua and Rhai support optional in `Cargo.toml`.

## [0.3.1] - 2025-01-06

### Changed

- `README.md`.
- Updated `smart-patcher` dependency.

## [0.3.0] - 2025-01-06

### Added

- `UseFromStorage` Actions.
- `Info` validation.
- Relative paths validation in project's setup.
- `ls content` and `new content` commands.

### Fixed

- Translations.

## [0.2.1] - 2025-01-04

### Added

- `Patch` Actions.

### Changed

- `README.md` and `DOCS.ru.md`.

### Fixed

- One translation in `i18n-ru`.

## [0.2.0] - 2024-12-27

### Added

- Russian language support.

### Fixed

- Build saves.

## [0.1.1-beta-14] - 2024-12-24

### Added

- Support of Pipeline-specific build folders (`exclusive_exec_tag`).

### Changed

- Push some edit variants to the top in TUI.

## [0.1.1-beta-13] - 2024-12-24

### Added

- `DEPLOYER_TRIM_ERR_OUT_LINES` environment variable to cut error's log.
- Initial values to edit TUI.
- Checks on empty project config.
- `Ctrl+C` support.

### Changed

- Writing build folder from every Pipeline that running.
- `Observe` Action now runs without piping the output.

### Fixed

- Local time in logs.
- Project names with `/` support.

## [0.1.1-beta-12] - 2024-12-18

### Added

- Allowed specifying build folder at `build` command.

### Changed

- Updated `DOCS.ru.md`.

## [0.1.1-beta-11] - 2024-12-18

### Added

- Build's file log.

### Changed

- `silent` option overrides `verbose` mode.
- Several Pipelines are available to be specified in `build` command.

### Fixed

- `README.md`

## [0.1.1-beta-10] - 2024-12-18

### Added

- `current` and `no-pipe` options to `build` command.

### Changed

- Some default prompts in TUI.

### Fixed

- Empty descriptions in `ls` command.

## [0.1.1-beta-9] - 2024-12-17

### Added

- Custom shell support by `DEPLOYER_SH_PATH` environment variable.
- Russian documentation inside `DOCS.ru.md`.

### Fixed

- `Check` Action's validation and prompt setup.
- A lot of TUI issues.

## [0.1.1-beta-4] - 2024-12-11

### Added

- `Interrupt` Action.
- `show_success_output` and `show_bash_c` options.
- Edit TUI to all Deployer entities.

### Fixed

- Deployer's self-installation.

## [0.1.0-alpha-11] - 2024-12-03

### Fixed

- Cache files.

## [0.1.0-alpha-10] - 2024-12-03

### Added

- Default project Pipelines.

## [0.1.0-alpha-9] - 2024-12-03

### Fixed

- Error outputs.

## [0.1.0-alpha-8] - 2024-12-03

### Added

- GitHub CI.

### Changed

- Exit on project's empty Pipeline list.

### Fixed

- Configuration saving.

### Removed

- YAML and TOML support (due to bug in deserialization from complex structures).

## [0.1.0-alpha-7] - 2024-12-03

### Added

- Pipelines and Actions.
- Build support.
- TUI.

[1.4.0-beta-2]:   https://github.com/impulse-sw/deployer/compare/1.4.0-beta-1...1.4.0-beta-2
[1.4.0-beta-1]:   https://github.com/impulse-sw/deployer/compare/1.3.5...1.4.0-beta-1
[1.3.5]:          https://github.com/impulse-sw/deployer/compare/1.3.4...1.3.5
[1.3.4]:          https://github.com/impulse-sw/deployer/compare/1.3.3...1.3.4
[1.3.3]:          https://github.com/impulse-sw/deployer/compare/1.3.2...1.3.3
[1.3.2]:          https://github.com/impulse-sw/deployer/compare/1.3.1...1.3.2
[1.3.1]:          https://github.com/impulse-sw/deployer/compare/1.3.0...1.3.1
[1.3.0]:          https://github.com/impulse-sw/deployer/compare/1.2.1...1.3.0
[1.2.1]:          https://github.com/impulse-sw/deployer/compare/1.2.0...1.2.1
[1.2.0]:          https://github.com/impulse-sw/deployer/compare/1.1.0-beta-1...1.2.0
[1.1.0-beta-2]:   https://github.com/impulse-sw/deployer/compare/1.1.0-beta-1...1.1.0-beta-2
[1.1.0-beta-1]:   https://github.com/impulse-sw/deployer/compare/1.0.1...1.1.0-beta-1
[1.0.1]:          https://github.com/impulse-sw/deployer/compare/1.0.0...1.0.1
[1.0.0]:          https://github.com/impulse-sw/deployer/compare/0.3.4...1.0.0
[0.3.4]:          https://github.com/impulse-sw/deployer/compare/0.3.3...0.3.4
[0.3.3]:          https://github.com/impulse-sw/deployer/compare/0.3.2...0.3.3
[0.3.2]:          https://github.com/impulse-sw/deployer/compare/0.3.1...0.3.2
[0.3.1]:          https://github.com/impulse-sw/deployer/compare/0.3.0...0.3.1
[0.3.0]:          https://github.com/impulse-sw/deployer/compare/0.2.1...0.3.0
[0.2.1]:          https://github.com/impulse-sw/deployer/compare/0.2.0...0.2.1
[0.2.0]:          https://github.com/impulse-sw/deployer/compare/0.1.1-beta-14...0.2.0
[0.1.1-beta-14]:  https://github.com/impulse-sw/deployer/compare/0.1.1-beta-13...0.1.1-beta-14
[0.1.1-beta-13]:  https://github.com/impulse-sw/deployer/compare/0.1.1-beta-12...0.1.1-beta-13
[0.1.1-beta-12]:  https://github.com/impulse-sw/deployer/compare/0.1.1-beta-11...0.1.1-beta-12
[0.1.1-beta-11]:  https://github.com/impulse-sw/deployer/compare/0.1.1-beta-10...0.1.1-beta-11
[0.1.1-beta-10]:  https://github.com/impulse-sw/deployer/compare/0.1.1-beta-9...0.1.1-beta-10
[0.1.1-beta-9]:   https://github.com/impulse-sw/deployer/compare/0.1.1-beta-4...0.1.1-beta-9
[0.1.1-beta-4]:   https://github.com/impulse-sw/deployer/compare/0.1.0-alpha-11...0.1.1-beta-4
[0.1.0-alpha-11]: https://github.com/impulse-sw/deployer/compare/0.1.0-alpha-10...0.1.0-alpha-11
[0.1.0-alpha-10]: https://github.com/impulse-sw/deployer/compare/0.1.0-alpha-9...0.1.0-alpha-10
[0.1.0-alpha-9]:  https://github.com/impulse-sw/deployer/compare/0.1.0-alpha-8...0.1.0-alpha-9
[0.1.0-alpha-8]:  https://github.com/impulse-sw/deployer/compare/0.1.0-alpha-7...0.1.0-alpha-8
[0.1.0-alpha-7]:  https://github.com/impulse-sw/deployer/commits/0.1.0-alpha-7
