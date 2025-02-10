//! Containered module.
//!
//! Contains code for generating `Dockerfile`s and perform build and run your projects.

use std::fs;
use std::io::Write;

use crate::configs::DeployerProjectOptions;
use crate::entities::containered_opts::ContaineredOpts;
use crate::entities::custom_command::CustomCommand;
use crate::entities::environment::RunEnvironment;
use crate::entities::traits::Execute;
use crate::i18n;
use crate::pipelines::DescribedPipeline;

pub const BASE_IMAGE: &str = "ubuntu:latest";

#[cfg(feature = "python")]
pub const PREFLIGHT_DEFAULT: &str =
  "RUN apt-get update && apt-get install -y python3-dev && rm -rf /var/lib/apt/lists/*";
#[cfg(not(feature = "python"))]
pub const PREFLIGHT_DEFAULT: &str = "";

#[cfg(feature = "python")]
pub const DEPLOYER_DEFAULT_PREFLIGHT: &str =
  "RUN apt-get update && apt-get install -y build-essential curl git python3-dev && rm -rf /var/lib/apt/lists/*";
#[cfg(not(feature = "python"))]
pub const DEPLOYER_DEFAULT_PREFLIGHT: &str =
  "RUN apt-get update && apt-get install -y build-essential curl git && rm -rf /var/lib/apt/lists/*";

pub const GENERIC_DOCKERFILE: &str = r#"# generated file
FROM {deployer-base-image} AS deployer-builder
WORKDIR /app
{preflight-install-deployer-deps}
{deployer-build-cmds}

FROM {base-image} AS deployer-executor
WORKDIR /app
{preflight-commands}
COPY --from=deployer-builder /app/deployer/target/release/deployer .
{run-strategy}
CMD ["/app/deployer", "run", "{pipeline-name}", "--current", "--containered"{no-pipe}]

"#;

fn generate_dockerignore(env: &RunEnvironment, config: &DeployerProjectOptions) -> anyhow::Result<()> {
  let mut cache_files = config
    .cache_files
    .iter()
    .map(|el| el.to_str().unwrap().to_string())
    .collect::<Vec<_>>();
  cache_files.sort();
  let cache_files = cache_files.join("\n");
  let mut dockerignore = fs::File::options()
    .create(true)
    .write(true)
    .truncate(true)
    .open(env.run_dir.join(".dockerignore"))
    .map_err(|e| anyhow::anyhow!(format!("Can't open `.dockerignore` due to: {}", e)))?;
  dockerignore.write_all(cache_files.as_bytes())?;
  Ok(())
}

fn generate_dockerfile(
  env: &RunEnvironment,
  pipeline: &DescribedPipeline,
  opts: &ContaineredOpts,
  exclusive_exec_tag: &str,
) -> anyhow::Result<()> {
  let resulting_image = GENERIC_DOCKERFILE
    .replace(
      "{deployer-base-image}",
      opts.build_deployer_base_image.as_deref().unwrap_or(BASE_IMAGE),
    )
    .replace(
      "{preflight-install-deployer-deps}",
      opts
        .preflight_deployer_build_deps
        .as_deref()
        .unwrap_or(DEPLOYER_DEFAULT_PREFLIGHT),
    )
    // Deployer needs `nightly` toolchain by default, so we install it on default image - `ubuntu:latest`.
    // Consider to install this toolchain by yourself by specifying `deployer_build_cmds` parameter,
    // if you want to change Deployer's build base image.
    .replace(
      "{deployer-build-cmds}",
      &opts
        .deployer_build_cmds
        .as_ref()
        .map(|v| v.join("\n"))
        .unwrap_or([
          "RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --profile minimal --default-toolchain nightly",
          r#"ENV PATH="/root/.cargo/bin:${PATH}""#,
          "RUN git clone --single-branch --branch unstable https://github.com/impulse-sw/deployer.git && cd deployer && cargo build --release",
        ].join("\n")),
    )
    .replace("{base-image}", opts.base_image.as_deref().unwrap_or(BASE_IMAGE))
    .replace(
      "{preflight-commands}",
      &opts
        .preflight_cmds
        .as_ref()
        .map(|v| v.join("\n"))
        .unwrap_or(PREFLIGHT_DEFAULT.to_string()),
    )
    .replace(
      "{run-strategy}",
      &opts.concat_strategies().unwrap_or("COPY . .".to_string()),
    )
    .replace("{pipeline-name}", &pipeline.title)
    .replace("{no-pipe}", if env.no_pipe { r#", "--no-pipe""# } else { "" });
  let filepath = env.run_dir.join(format!("Dockerfile.{}", exclusive_exec_tag));
  let mut dockerfile = fs::File::options()
    .create(true)
    .write(true)
    .truncate(true)
    .open(&filepath)
    .map_err(|e| {
      anyhow::anyhow!(format!(
        "Can't open `Dockerfile.{}` due to: `{}`; filepath: {:?}",
        exclusive_exec_tag, e, filepath
      ))
    })?;
  dockerfile.write_all(resulting_image.as_bytes())?;
  Ok(())
}

pub fn execute_pipeline_containered(
  config: &DeployerProjectOptions,
  env: &RunEnvironment,
  pipeline: &DescribedPipeline,
) -> anyhow::Result<()> {
  let canonicalized = env.run_dir.canonicalize()?;
  let canonicalized = canonicalized.to_str().expect("Can't convert `Path` to string!");
  if !env.silent_build {
    println!("{}: {}", i18n::BUILD_PATH, canonicalized);
  }

  let exclusive_exec_tag = pipeline.exclusive_exec_tag.clone().unwrap_or(String::from("default")) + "-containered";
  let opts = pipeline.containered_opts.as_ref().unwrap();
  opts.sync_fake_content(env)?;
  generate_dockerfile(env, pipeline, opts, &exclusive_exec_tag)?;
  generate_dockerignore(env, config)?;
  println!("Started `{}/{}` image build...", config.project_name, pipeline.title);

  if !(CustomCommand {
    bash_c: format!(
      "sudo docker build {}-t {}/{} -f Dockerfile.{} .",
      if env.new_build { "--no-cache " } else { "" },
      config.project_name,
      pipeline.title,
      exclusive_exec_tag
    ),
    ignore_fails: false,
    only_when_fresh: None,
    placeholders: None,
    remote_exec: None,
    replacements: None,
    show_bash_c: true,
    show_success_output: true,
    daemon: None,
  })
  .execute(&RunEnvironment {
    no_pipe: true,
    daemons: env.daemons.clone(),
    ..(*env)
  })?
  .0
  {
    panic!("Image wasn't build!")
  }
  println!("Image was built successfully.");

  let volume_path = env.artifacts_dir.join(&pipeline.title);

  if !(CustomCommand {
    bash_c: format!(
      "sudo docker run -v {:?}:/app/artifacts {}/{}",
      volume_path, config.project_name, pipeline.title
    ),
    ignore_fails: false,
    only_when_fresh: None,
    placeholders: None,
    remote_exec: None,
    replacements: None,
    show_bash_c: true,
    show_success_output: true,
    daemon: None,
  })
  .execute(&RunEnvironment {
    no_pipe: true,
    daemons: env.daemons.clone(),
    ..(*env)
  })?
  .0
  {
    panic!("Deployer didn't run!")
  }

  Ok(())
}
