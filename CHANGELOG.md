<!-- markdownlint-disable-file MD024 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
