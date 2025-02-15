# deployer

Deployer is a relatively simple, yet powerful localhost CI/CD instrument. It allows you to:

- have your own actions and pipelines repositories (`Actions Registry` and `Pipelines Registry`) in a single JSON file
- create actions and pipelines from TUI or using JSON/YAML/TOML configuration files
- configure actions for specific project
- satisfy requirements for your system to run pipelines
- check compatibility over actions and projects
- run actions and pipelines on remote hosts (you need to setup your remote access using SSH key, also you need to install `deployer` on remote host)
- use variables for commands from `env`-files and HashiCorp Vault KV2-storage
- run pipelines with different cache requirements in different build folders
- run pipelines inside containerized environments (Docker by default) with cache strategies
- store common content in Deployer's storage, add and patch additional files for build on the fly
- and share your project build/deploy settings very quickly and without any dependencies.

## Migration guide

If you encounter problems after updating Deployer, please read [Migration Guide](MIGRATIONS.md).

## Build

Well, the building process is very easy. You need to install Rust first:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, execute this:

```bash
git clone https://github.com/impulse-sw/deployer.git
cd deployer
cargo install --path .                     # to install English version
cargo install --path . --features=i18n-ru  # to install Russian version

# if you already have Deployer installed
deployer run en  # to install English version
deployer run ru  # to install Russian version
```

That's it! Now you have `/home/username/.cargo/bin/deployer` binary. Modify the `PATH` variable, if you need to.

## Usage

All project configuration is stored in `deploy-config.json` or `.deploy-config.json` file.

First of all, let's create a simple action.

```bash
deployer new action
```

For example, let's name it `UPX Compress`. The short name will be `upx-compress`, the version - `0.1.0`.

The full JSON is:

```json
{
  "title": "UPX Compress",
  "desc": "Compress the binary file with UPX.",
  "info": "upx-compress@0.1.0",
  "tags": [
    "upx"
  ],
  "action": {
    "type": "post_build",
    "supported_langs": [
      "Rust",
      "Go",
      "C",
      "Cpp",
      "Python",
      {
        "Other": "any"
      }
    ],
    "commands": [
      {
        "bash_c": "upx <artifact>",
        "placeholders": [
          "<artifact>"
        ],
        "ignore_fails": false,
        "show_success_output": false,
        "show_bash_c": false
      }
    ]
  },
  "requirements": [
    {
      "type": "exists_any",
      "paths": [
        "/usr/bin/upx",
        "~/.local/bin/upx"
      ]
    }
  ]
}
```

If you're not familiar with UPX, consider visiting it's [home page](https://upx.github.io/).

So, let's create a pipeline that will build the binary from the Rust code with preinstalled `cargo-rel@0.1` action and then compress this binary with `upx-compress@0.1.0`.

```bash
deployer new pipeline
```

The full JSON is:

```json
{
  "title": "Rust Enhanced Pipeline",
  "desc": "Build the Rust project with Cargo.",
  "info": "rust-default@0.1.0",
  "tags": [
    "cargo",
    "clippy",
    "build",
    "upx"
  ],
  "actions": [
    {
      "title": "Lint",
      "desc": "Got from `Cargo Clippy`.",
      "info": "cargo-clippy@0.1.0",
      "tags": [
        "cargo",
        "clippy"
      ],
      "action": {
        "type": "pre_build",
        "supported_langs": [
          "rust"
        ],
        "commands": [
          {
            "bash_c": "cargo clippy",
            "ignore_fails": false,
            "show_success_output": true,
            "show_bash_c": true
          }
        ]
      }
    },
    {
      "title": "Build",
      "desc": "Got from `Cargo Build (Release)`. Build the Rust project with Cargo default settings in release mode",
      "info": "cargo-rel@0.1",
      "tags": [
        "rust",
        "cargo"
      ],
      "action": {
        "type": "build",
        "supported_langs": [
          "rust"
        ],
        "commands": [
          {
            "bash_c": "cargo build --release",
            "ignore_fails": false,
            "show_success_output": false,
            "show_bash_c": true
          }
        ]
      }
    },
    {
      "title": "Compress",
      "desc": "Got from `UPX Compress`.",
      "info": "upx@0.1.0",
      "tags": [
        "upx"
      ],
      "action": {
        "type": "post_build",
        "supported_langs": [
          "any"
        ],
        "commands": [
          {
            "bash_c": "upx <artifact>",
            "placeholders": [
              "<artifact>"
            ],
            "replacements": [
              {
                "group": [
                  {
                    "from": "<artifact>",
                    "to": {
                      "title": "target/release/deployer",
                      "is_secret": false,
                      "value": {
                        "type": "plain",
                        "value": "target/release/deployer"
                      }
                    }
                  }
                ]
              }
            ],
            "ignore_fails": false,
            "show_success_output": false,
            "show_bash_c": false
          }
        ]
      }
    }
  ],
  "default": true
}
```

Note that you can change the inner content of Actions inside Pipelines, and also can change the inner content of Pipelines and their Actions if these Pipelines assigned to your project. The changes will not affect Actions and Pipelines from Deployer's Registries.

You can view your Actions and Pipelines and get it in JSON by simple commands:

```bash
deployer ls actions
deployer ls pipelines

deployer cat action upx-compress@0.1.0
deployer cat pipeline rust-default@0.1.0
```

And, of course, load Actions and Pipelines from JSON files by:

```bash
deployer new action -f {your config}
```

The next step is to init the project and assign the `rust-default@0.1.0` Pipeline to it.

```bash
cd my-rust-project
deployer init
deployer with rust-default@0.1.0
```

Deployer will ask you to specify some things (e.g., targets - for this project and `rust-default@0.1.0` Pipeline it will be `target/release/deployer`). After all you will get this `deploy-config.json`:

```json
{
  "project_name": "my-rust-project",
  "langs": [
    "rust"
  ],
  "targets": [
    {
      "arch": "x86_64",
      "os": "Linux",
      "os_derivative": "any",
      "os_version": {
        "type": "no"
      }
    }
  ],
  "cache_files": [
    "Cargo.lock",
    "target"
  ],
  "pipelines": [
    {
      "title": "build-and-compress",
      "desc": "Got from `Rust Enhanced Pipeline`. Build the Rust project with Cargo.",
      "info": "rust-default@0.1.0",
      "tags": [],
      "actions": [
        {
          "title": "Build the project.",
          "desc": "Got from `Cargo Build (Release)`. Build the Rust project with Cargo default settings in release mode",
          "info": "cargo-rel@0.1",
          "tags": [
            "rust",
            "cargo"
          ],
          "action": {
            "type": "build",
            "supported_langs": [
              "Rust"
            ],
            "commands": [
              {
                "bash_c": "cargo build --quiet --release",
                "ignore_fails": false,
                "show_success_output": false,
                "show_bash_c": false
              }
            ]
          }
        },
        {
          "title": "Compress the resulting binary.",
          "desc": "Got from `UPX Compress`. Compress the binary file with UPX.",
          "info": "upx-compress@0.1.0",
          "tags": [],
          "action": {
            "type": "post_build",
            "supported_langs": [
              "any"
            ],
            "commands": [
              {
                "bash_c": "upx <artifact>",
                "placeholders": [
                  "<artifact>"
                ],
                "replacements": [
                  {
                    "group": [
                      {
                        "from": "<artifact>",
                        "to": {
                          "title": "target/release/my-rust-project",
                          "is_secret": false,
                          "value": {
                            "type": "plain",
                            "value": "target/release/my-rust-project"
                          }
                        }
                      }
                    ]
                  }
                ],
                "ignore_fails": false,
                "show_success_output": false,
                "show_bash_c": false
              }
            ]
          },
          "requirements": [
            {
              "type": "exists_any",
              "paths": [
                "/usr/bin/upx",
                "~/.local/bin/upx"
              ]
            }
          ]
        }
      ]
    }
  ],
  "artifacts": [
    "target/release/my-rust-project"
  ],
  "variables": [],
  "place_artifacts_into_project_root": [
    {
      "from": "target/release/deployer",
      "to": "deployer"
    }
  ],
  "version": 4
}
```

Having only `deploy-config.json` inside your project's root, you can share your build/deploy configurations.

If you want to hide your configuration, you can rename it to `.deploy-config.json` manually.

At the end, let's build the project!

```bash
deployer run

# see the run options: you can share cache files and folders by symlinking or copying
deployer run --help
deployer run -fc

# or explicitly specify the project pipeline's short name - `build-and-compress`
deployer run build-and-compress
```

For other options, check:

```bash
deployer run -h
```
