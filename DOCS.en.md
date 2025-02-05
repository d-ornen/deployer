# Deployer: documentation for version `1.4.X`

The actual documentation is available with `deployer docs`.

The main Deployer configuration format is JSON, but it also supports YAML and TOML. You can initialize the project with the `-F` flag and specify the preferred format, or edit the global configuration (which will remain as JSON for now) by specifying the `preferred_conf_format` (`yaml`/`toml`/`json`) field, or ask the Deployer to save the configuration in a different format via the command `deployer edit project`. All the examples in the documentation below are written in JSON.

## Description of working principles

Deployer is, at its core, a local CI/CD. In other words, a `bash` command manager.

Typically, it runs in a separate folder to save the cache while keeping the code folder clean. However, you can specify either any folder or the code folder; if you already have caches, you can copy them from the source folder, symlink to them, or ignore them completely and run from scratch.

## Description of the main entities

### 1. `Action`

Action is the main entity of Deployer. Actions as part of Pipelines are used to build, install, and deploy processes. However, an Action itself cannot be assigned to a project, that's what Pipelines are for (see below).

As part of Pipelines or in the Deployer's Action Registry, an Action looks like a construction:

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
      "any"
    ],
    "commands": [
      {
        "bash_c": "upx <artifact>",
        "placeholders": [
          "<artifact>"
        ],
        "ignore_fails": false,
        "show_success_output": false,
        "show_bash_c": false,
        "only_when_fresh": false
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

As part of Registries, each Action and each Pipeline are values in a dictionary with the `info` key (e.g. `"upx-compress@0.1.0": { ... }`). Thus they can be quickly edited, their contents displayed, added to Pipelines and projects.

For each Action within a Pipeline, a list of `requirements` can be assigned. These will be checked before each Pipelines run, and if at least one requirement is not met, the Pipeline will not be executed. A requirement can be set in three ways:

```json,ignore
[
  {
    // if one of these paths will be found, the requirement will be considered satisfied
    "type": "exists_any",
    "paths": [
      "/usr/bin/upx",
      "~/.local/bin/upx"
    ]
  },
  {
    // if this path exists, the requirement is considered satisfied
    "type": "exists",
    "path": "/usr/bin/mold"
  },
  {
    // if this check is passed, the requirement will be considered satisfied (for details, see below - Action `Check`)
    "type": "check_success",
    "command": {
      "bash_c": "/usr/bin/python -V",
      "ignore_fails": true,
      "show_success_output": false,
      "show_bash_c": false,
      "only_when_fresh": false
    },
    "success_when_found": "Python 3."
  },
  {
    // if a given remote host exists in the Registry, is accessible, and its Deployer version is identical to the version of the running Deployer,
    // the requirement will be considered satisfied
    "type": "remote_accessible_and_ready",
    "remote_host_name": "short-name"
  }
]
```

There are 3 categories of basic Actions and 9 additional types of Actions:

1. Build Actions (`pre_build`, `build`, `post_build` and `test`)
2. Install Actions (`pack`, `deliver`, `install`)
3. Deploy Actions (`configure_deploy`, `deploy`, `post_deploy`)
4. `observe` action
5. `interrupt` action
6. Action with custom command `custom`
7. Action of checking the output of the custom command `check`
8. Action of adding content to the Deployer's storage `add_to_storage` and using this content `use_from_storage`
9. The action of applying a `patch`
10. Actions of synchronization build folders - from current to remote host `sync_to_remote` and vice versa `sync_from_remote`

The concept of a custom command, a command for the terminal shell, is fundamental. The `custom`, `observe`, and the three main categories of Actions contain one or more custom commands inside.

#### 1.1. Custom Command

The command description for Deployer is as follows:

```json
{
  "bash_c": "upx <artifact>",
  "placeholders": [
    "<artifact>"
  ],
  "ignore_fails": false,
  "show_success_output": false,
  "show_bash_c": false,
  "only_when_fresh": false,
  "remote_exec": []
}
```

- `bash_c` contains the text of the command to be executed in the terminal
- `placeholders` contains a list of`placeholders` that can be replaced with project variables and artifacts to perform necessary actions with them
- `ignore_fails` tells Deployer whether to qualify a process output status not equal to zero as normal command behavior or not; if not, Deployer will abort Pipeline execution and exit with status `1`
- `show_success_output` tells Deployer whether to print the command output always (including when the process exit status is `0`), or whether to print only on error
- `show_bash_c` tells the Deployer whether to print the full command text on the screen; this can be useful when the command contains vulnerable variables
- `only_when_fresh` tells Deployer that this action should only be performed on a fresh build (either on the first build, or when explicitly instructed to rebuild from scratch with the `-f` option)
- `remote_exec` contains a list of short hostnames on which this command will need to be executed

When a command is specialized for a particular project, it gains an additional property - `replacements`:

```json
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
  "show_bash_c": false,
  "only_when_fresh": false
}
```

`replacements` contains a list of replacements of placeholders in the command with the specified artifacts or variables (see p.3). Note that the same command can be executed multiple times for different sets of variables, even if specified once in an Action:

```json
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
    },
    {
      "group": [
        {
          "from": "<artifact>",
          "to": {
            "title": "target/release/another",
            "is_secret": false,
            "value": {
              "type": "plain",
              "value": "target/release/another"
            }
          }
        }
      ]
    }
  ],
  "ignore_fails": false,
  "show_success_output": false,
  "show_bash_c": false,
  "only_when_fresh": false
}
```

In the above example only one placeholder `<artifact>` is used, but there can be several of them, including different options for executing the command.

Accordingly, if you just want to execute commands that cannot be assigned to one of the three main types of Actions, you should use an Action of type `custom`:

```json
{
  "title": "List all files and folders",
  "desc": "",
  "info": "ls@0.1.0",
  "tags": [],
  "action": {
    "type": "custom",
    "bash_c": "ls",
    "ignore_fails": false,
    "show_success_output": true,
    "show_bash_c": true,
    "only_when_fresh": false
  }
}
```

#### 1.2. Build Actions - `pre_build`, `build` and `post_build`

For Build Actions, specialization in programming languages is specific: depending on whether the set of languages used in the project matches the set specified in the Build Action, Deployer will warn you about using Actions that are incompatible with the project.

> [!NOTE]
> Specializations only work when assigning Actions or Pipelines from the TUI. If you manually edit the configuration by adding an incompatible Pipeline, the Deployer will not issue any warnings. This reflects the loose and advisory nature of such warnings, in contrast to `requirements`.

In the below example, we see an action that should be executed after the build:

```json
{
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
      "ignore_fails": false,
      "show_success_output": false,
      "show_bash_c": false,
      "only_when_fresh": false
    }
  ]
}
```

#### 1.3. Installation Actions - `pack`, `deliver` and `install`

For this group of Actions, the key specialization factor is the *target* object of the installation. If the characteristics of the project target - hardware or software platform - do not match the characteristics of the Installation Action, a warning will be issued.

We are happy to note that UPX refers to the Packaging Action rather than the Post-Build Action:

```json
{
  "title": "UPX Pack",
  "desc": "Pack the binary by UPX.",
  "info": "upx-pack@0.1.0",
  "tags": [
    "upx"
  ],
  "action": {
    "type": "pack",
    "target": {
      "arch": "x86_64",
      "os": "Linux",
      "os_derivative": "any",
      "os_version": {
        "type": "no"
      }
    },
    "commands": [
      {
        "bash_c": "upx <af>",
        "placeholders": [
          "<af>"
        ],
        "ignore_fails": false,
        "show_success_output": false,
        "show_bash_c": false,
        "only_when_fresh": false
      }
    ]
  }
}
```

- `arch` is a string designation for the target hardware architecture
- `os` is one of the variants (`android|ios|linux|unix-{unix-name}|windows|macos`) or any other string designation of the operating system.
- `os_derivative` is an additional description of the operating system or software platform
- `os_version` is the version of the operating system or software platform.

If `os_derivative` is missing, it is recommended to write `any`.

#### 1.4. Deployment Actions - `configure_deploy`, `deploy` and `post_deploy`

For this group of Actions, the key specialization factor is the deployment tulkit - Docker, Docker Compose, Podman, k8s or other containerization or virtualization toolkit. If the wrong tulkit is specified in the project, Deployer will issue a warning.

Here is an example with Docker Compose:

```json
{
  "title": "Build Docker Compose Image",
  "desc": "Build Docker image with Docker Compose",
  "info": "docker-compose-build@0.1.0",
  "tags": [
    "docker",
    "compose"
  ],
  "action": {
    "type": "configure_deploy",
    "deploy_toolkit": "docker-compose",
    "commands": [
      {
        "bash_c": "docker compose build",
        "ignore_fails": false,
        "show_success_output": false,
        "show_bash_c": true,
        "only_when_fresh": false
      }
    ]
  }
}
```

#### 1.5. The actions of adding `add_to_storage` content, using `use_from_storage` content, and applying a `patch`

Often projects can be sufficiently templated that the same files are copied between projects, but not modified and only required during build or deployment. Such files can be located in a special folder with relative paths preserved and added to the Deployer repository:

```bash
deployer new content
```

Then a new Action - `use_from_storage` - can be added to the Build Pipeline for projects that need to use these files:

```json
{
  "title": "Sync content",
  "desc": "",
  "info": "content-sync@0.1.0",
  "tags": [],
  "action": {
    "type": "use_from_storage",
    "content_info": "test-dockerfile-content@0.1.0"
  }
}
```

This will eventually add the content you need to the build folder when the Pipeline is executed.

Time after time, you will start to notice that some projects are overused in other projects as dependencies and need to be published somewhere. Package repositories are the best place for this, but if you don't want to publish your project, you can add it to the Deployer repository as content. Moreover, you can add it automatically using the `add_to_storage` action:

```json
{
  "title": "Add content",
  "desc": "",
  "info": "content-add@0.1.0",
  "tags": [],
  "action": {
    "type": "add_to_storage",
    "short_name": "my-project",
    "auto_version_rule": {
      "plain_file": "file-with-current-version.txt"
    }
  }
}
```

- `short_name` - string designation of the content, which will be used to place it in the storage and each time it is used
- `auto_version_rule` - a way to automatically determine the version of the content (either `plain_file` - a file that will contain only the version and nothing else, or `cmd_stdout` - a command that will display only the version and nothing else)

However, sometimes the file needs to be edited in some way - and not so much even the added content from the Deployer repository, but, for example, various files in the build dependencies, e.g. manually forking Python libraries to add the desired functionality, etc., etc. And, as a rule, you want to do it without creating forks and synchronizing changes with the main repository! You can't do it with `git` patches alone.

For this purpose, Deployer uses the [`smart-patcher`](https://github.com/impulse-sw/smart-patcher) library for patches. Such patches allow source code, complex documents, and even binary files to be modified, allowing you to search for necessary inclusions in the content based on sifting rules and even using scripts in languages such as Python, Lua, and Rhai. For example, the `smart-patcher` repository has an example with a patch for a Microsoft Word document - and many more examples.

To use smart patches you need to write a patch file first. Example:

```json
{
  "patches": [
    {
      "files": [
        {
          "just": "test_v5.docx"
        }
      ],
      "decoder": {
        "python": "../tests/test_v5.py"
      },
      "encoder": {
        "python": "../tests/test_v5.py"
      },
      "path_find_graph": [],
      "replace": {
        "from_to": [
          "game",
          "rock"
        ]
      }
    }
  ]
}
```

The action of a patch looks like this:

```json
{
  "title": "Apply patch",
  "desc": "",
  "info": "my-patch@0.1.0",
  "tags": [],
  "action": {
    "type": "patch",
    "patch": "my_path.json"
  }
}
```

The patch *should be located in the build folder* when you run Pipeline. A very good practice is to write patches and place them as content in the Deployer repository. Then both the patch file and the scripts will be located side by side and will be added during the build process.

When a patch is applied, Deployer displays the number of times it has been applied in the project. If the patch has not been applied once during the Pipeline process, *Deployer will generate an error*.

> [!NOTE]
> Deployer don't support patch scripts written in Python by default. If you need this, build Deployer with `deployer run en-full` command.

#### 1.6. Actions of synchronization build folders - from current to remote host `sync_to_remote` and vice versa `sync_from_remote`

Sometimes you need to synchronize build files between remote hosts and the current host. For example, when some actions must be performed on one host, and some on another. To do this, you can use the built-in Actions `sync_to_remote` and `sync_from_remote`:

```json
{
  "title": "Send build folder to remote",
  "desc": "",
  "info": "send-to-remote@0.1.0",
  "tags": [],
  "action": {
    "type": "sync_to_remote",
    "remote_host_name": "remote-pc"
  }
}
```

#### 1.7. Other actions - `interrupt`, `observe` and `test`

> [!NOTE]
> Don't have the configuration example you need? Create the action yourself using the `deployer new action` command and display it using the `deployer cat action my-action@x.y.z`.

`interrupt` is used to manually interrupt the build/deployment of a project. When Deployer reaches this action, it waits for user input to continue when you perform the necessary manual actions.

`observe` is an action that is almost identical to `custom`. It is used, for example, to start Prometheus, Jaeger or anything else. The distinctive feature is that it runs without I/O redirection, i.e. you can interact with programs in it.

And `test` is a special action that allows you to check what the command outputs to `stdout/stderr`:

```json
{
  "type": "test",
  "command": {
    "bash_c": "<af>",
    "placeholders": [
      "<af>"
    ],
    "ignore_fails": true,
    "show_success_output": false,
    "show_bash_c": false,
    "only_when_fresh": false
  },
  "success_when_found": "some rust regex"
}
```

- `success_when_found` tells Deployer that if it finds the specified regular expression, the execution of the command will be considered successful
- `success_when_not_found` tells Deployer that if it does not find the specified regular expression, the command execution will be considered successful.

Moreover, if both fields are specified, the execution will be considered successful if both options were successful (the first regular expression must find, the second must not find).

#### 1.8. Sub-pipelines

Sub-pipelines allow you to group Actions by description and purpose and use them together inside project Pipelines. An example:

```json
{
  "type": "sub_pipeline",
  "title": "Sub-pipeline running Action",
  "desc": "This is actual Pipeline inside your described Action.",
  "info": "test-subpipeline@0.2.0",
  "tags": [],
  "actions": [
    {
      "title": "List files",
      "desc": "Got from `List files and folders`.",
      "info": "ls@0.1.0",
      "tags": [],
      "action": {
        "type": "custom",
        "bash_c": "ls",
        "ignore_fails": false,
        "show_success_output": true,
        "show_bash_c": true,
        "only_when_fresh": false
      },
      "exec_in_project_dir": false
    }
  ]
}
```

> [!NOTE]
> Regardless of the presence of an exluisive tag, the Sub-pipeline will be made in the same folder as the parental Pipeline.

This concludes the description of Actions, and we move on to Pipelines.

### 2. `Pipeline`

A Pipeline is an ordered set of Actions that is necessary to achieve a certain goal. For example, when you need to check code quality, check the code with a static analyzer, then build, compress, package it for a certain distribution and upload it to hosting. Or when you need to build an Android application, sign it and install it on an ADB-connected device. The composition of Pipeline can be any, the main example is given in the `deploy-config.json` file of this repository:

```json
{
  "title": "Deployer Pipeline",
  "desc": "Default Deployer Pipeline for itself.",
  "info": "deployer-default@0.1.0",
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
    },
    {
      "title": "Install to ~/.cargo/bin",
      "desc": "",
      "info": "install-to-cargo-bin@0.1.1",
      "tags": [
        "cargo"
      ],
      "action": {
        "type": "install",
        "target": {
          "arch": "x86_64",
          "os": "Linux",
          "os_derivative": "any",
          "os_version": {
            "type": "no"
          }
        },
        "commands": [
          {
            "bash_c": "cp -f <artifact> ~/.cargo/bin",
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

In general, a Pipeline contains a list of Actions in the `actions` field.

In addition, if your Pipelines need to manage conflicting cache versions (for example, when building a project for different target architectures), you can specify an exclusive build tag in the `exclusive_exec_tag` field. For example, specify `x86_64` when adding a Pipeline build for one architecture and `aarch64` for another. Then Pipelines will be built in different folders and cache information will be saved in both cases.

#### 2.1. Containerized assembly and execution, as well as strategies

The deployment supports the build and execution of projects in containerized environments with the ability to extract artifacts to the project folder. This can be useful in cases where you need to build a project for other platforms and/or environments.

In addition, the Deployer allows you to specify basic images, a list of commands for installing dependencies and configuring the image, as well as strategies for saving caches. Collectively, this allows you to generate information for `Dockerfile` images automatically.

> [!NOTE]
> Deployer versions 1.4.0-beta-1/1.4.0-beta-2 only support Pipeline launch in Docker containerized environments.

Let's look at an example of a containerized Pipeline:

```json
{
  "title": "containered",
  "desc": "Got from `Deployer Pipeline`.",
  "info": "deployer-default@0.1.2",
  "tags": [
    "cargo",
    "clippy",
    "build"
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
            "bash_c": "cargo clippy --no-default-features --features=lua,rhai,tui,containered",
            "ignore_fails": false,
            "show_success_output": true,
            "show_bash_c": true
          }
        ]
      },
      "requirements": [
        {
          "type": "exists_any",
          "paths": [
            "/bin/cargo",
            "~/.cargo/bin/cargo"
          ]
        }
      ]
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
            "bash_c": "RUSTFLAGS='-Zthreads=16' cargo build --release --no-default-features --features=lua,rhai,tui,containered",
            "ignore_fails": false,
            "show_success_output": false,
            "show_bash_c": true
          }
        ]
      },
      "requirements": [
        {
          "type": "exists_any",
          "paths": [
            "/bin/cargo",
            "~/.cargo/bin/cargo"
          ]
        }
      ]
    }
  ],
  "default": false,
  "containered_opts": {
    "preflight_cmds": [
      "RUN apt-get update && apt-get install -y build-essential curl git && rm -rf /var/lib/apt/lists/*",
      "RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --profile minimal --default-toolchain nightly",
      "ENV PATH=\"/root/.cargo/bin:${PATH}\"",
      "RUN rustup component add clippy"
    ],
    "cache_strategies": [
      {
        "fake_content": "docker-fake-files@0.1.0",
        "copy_cmds": [
          "COPY rust-toolchain.toml .",
          "COPY .docker-fake-files/rust/lib.rs src/lib.rs",
          "COPY .docker-fake-files/rust/main.rs src/main.rs",
          "COPY Cargo.toml .",
          "COPY deploy-config.json ."
        ],
        "pre_cache_cmds": [
          "RUN /app/deployer run containered --current --containered --no-pipe"
        ]
      },
      {
        "copy_cmds": [
          "COPY src/ src/"
        ],
        "pre_cache_cmds": [
          "COPY DOCS.en.md .",
          "COPY DOCS.ru.md .",
          "RUN touch src/main.rs",
          "RUN touch src/lib.rs",
          "RUN /app/deployer run containered --current --containered --no-pipe"
        ]
      }
    ]
  },
  "exclusive_exec_tag": "containered"
}
```

The only difference is the addition of the `containered_opts` field, which automatically forces the Deployer to execute this Pipeline in a containerized environment.

- `base_image` - you can specify the base image for building the project (default is `ubuntu:latest`)
- `preflight_cmds` - a list of commands for installing the environment correctly
- `build_deployer_base_image`, `preflight_deployer_build_deps` and `deployer_build_cmds` - base image, configuration commands and commands for building the Deployer itself
- `cache_strategies` - caching strategies during build stage

Since the Deployer is needed in a containerized environment, check the compatibility of the Deployer and the environment by launching the Pipeline. If the Deployer is built with Python support, it is best to use identical base images.

To keep the build cache, rather than constantly rebuilding the Pipeline from scratch, it is recommended to specify caching strategies. They are executed when building a containerized environment. Available fields:

- `fake_content` - a field for syncing content to replace existing files (works the same way as `use_from_storage`, but does not support `latest` tags)
- `copy_cmds` - commands for copying source code to an image
- `pre_cache_cmds` - commands for pre-caching

Caching strategies are suitable for implementing multi-stage builds. In the above example, there is a two-stage build for a Rust project, which first requires copying the real `Cargo.toml` and fake `lib.rs `and `main.rs `to compile all the dependencies of the project first, and then copies the real source code `src/` and updates the timestamps `RUN touch src/main.rs & touch src/lib.rs` to then build the project without having to rebuild dependencies. In this case, the dependency cache will be used until `Cargo.toml` is edited.

To rebuild containered environment from scratch, run Deployer with flag `-f`/`--fresh`.

### <a id="other-entities">3. Other entities</a>

One of the most important entities are variables. They are both the keepers of your secrets and the very dynamic entities that can change the outcome of the Pipeline execution. Here is an example of a simple variable:

```json
{
  "title": "deployer artifact location",
  "is_secret": false,
  "value": {
    "type": "plain",
    "value": "target/release/deployer"
  }
}
```

- `title` - the name of the variable (how it will be displayed in the TUI)
- `is_secret` - whether the variable is a secret (if it is, the command that contains it will not be shown on the screen)
- `value` - the value of the variable itself or information about where and how to get this value from.

There are three types of variables supported now:

1. `plain` - the content of the string is the variable
2. `from_env_var` - the variable will be taken from Deployer's shell environment
3. `from_env_file` - the variable will be taken from the specified `env-file` with the specified key.
4. `from_hc_vault_kv2` - the variable will be taken from the HashiCorp Vault KV2 repository with the specified `mount_path` and `secret_path`

Examples:

```json
{
  "title": "Grafana token",
  "is_secret": true,
  "value": {
    "type": "from_env_file",
    "env_file_path": ".env",
    "key": "GRAFANA_TOKEN"
  }
}
```

```json
{
  "title": "Simple env var",
  "is_secret": false,
  "value": {
    "type": "from_env_var",
    "var_name": "variable-key"
  }
}
```

```json
{
  "title": "Secret!",
  "is_secret": true,
  "value": {
    "type": "from_hc_vault_kv2",
    "mount_path": "The mount path where your KV2 secrets engine is mounted",
    "secret_path": "Path to your secret"
  }
}
```

Note that you must specify two environment variables before using `from_hc_vault_kv2` variables: the `DEPLOYER_VAULT_ADDR` (Vault URL) and `DEPLOYER_VAULT_TOKEN` (Vault token).

Another important entity is the remote host. The deployer stores all hosts in the Registry (global configuration file - list `remote_hosts`). The host structure looks like this:

```json
{
  "short_name": "localhost",
  "ip": "127.0.0.1",
  "port": 22,
  "username": "username",
  "ssh_private_key_file": "/path/to/id_rsa"
}
```

To be able to use a host, before adding it, you must create a key and allow authorization on the remote host using the key.

## CLI Utility Description

Deployer is primarily a CLI utility. You can see help for any Deployer command by specifying the `-h` option. Here are some examples of the most common commands:

```bash
deployer new action                            # create an Action and put in Registry
deployer new pipeline                          # create a Pipeline and put in Registry
deployer new remote                            # add new remote host to Registry
deployer init                                  # init project, fill all attributes
deployer with                                  # check compatibility and assign Pipeline to project,
                                               # also specify needed variables and artifacts
deployer run                                   # run default Pipeline
deployer run my-pipe                           # run specified `my-pipe` Pipeline
deployer run configure,build -o build-folder   # run `configure` and `build` Pipelines in a `build-folder`
deployer run -R my-remote my-pipe              # run `my-pipe` Pipeline on remote host `my-remote`
```

### Console Interface (TUI)

Deployer has support for a high-end terminal-based customizer, allowing you to forget about manually writing Actions and Pipelines for your projects. Just try to create an Action or Pipeline and Deployer will ask you about everything.

### Logs

In the Deployer build caches folder, there is a `logs` folder that contains project log files with the date and time of the build. The information in them repeats information from the terminal screen and does not *currently* store the entire log of each command execution in the shell.
