# `deployer`'s TODO list

- [ ] make requirement checks at remote hosts
- [ ] maybe, make some `env` variable to set Deployer to execute only local actions
- [ ] make `Transfer` Actions to sync only selected files to and from remote
- [ ] make `Artifact` docs inside DOCS.ru.md & DOCS.en.md
- [ ] make `deployer share content` command
- [ ] `TestAction` <-> `CheckAction` (replace 1 with 2 and remove 2)

## Containered builds

- [ ] make `deployer run --containered` option for builds inside the container (ignore `Observe` and other Actions)
- [ ] make `ContaineredOpts` with `Vec<RunStrategy>`
- [ ] make `ContaineredOpts.build_deployer: Option<bool>` & `ContaineredOpts.install_deployer_build_deps_cmds: Option<Vec<String>>` fields

Run strategy is a informational struct with:
  1. [ ] `copy_all_except: Option<Vec<String>>`
  2. [ ] `copy_stubs: Option<Vec<{ stub_content: String, copy_to: String, }>>`
  3. [ ]
