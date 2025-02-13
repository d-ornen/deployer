//! Containered module.
//!
//! Contains code for generating `Dockerfile`s and perform build and run your projects.

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::process::exit;

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
{cache-strategy}
CMD ["/app/deployer", "run", "{pipeline-name}", "--current", "--containered"{no-pipe}]

"#;

fn run_simple(env: &RunEnvironment, bash_c: String) -> anyhow::Result<()> {
  if !(CustomCommand {
    bash_c,
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
    anyhow::bail!("")
  }
  Ok(())
}

fn is_user_in_group(group_name: &str) -> anyhow::Result<bool> {
  let groups = match nix::unistd::getgroups() {
    Ok(groups) => groups,
    Err(e) => anyhow::bail!("Failed to get user groups: {}", e),
  };
  for gid in groups {
    if let Ok(Some(group)) = nix::unistd::Group::from_gid(gid)
      && group.name.as_str().eq(group_name)
    {
      return Ok(true);
    }
  }
  Ok(false)
}

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
      "{cache-strategy}",
      &opts.concat_strategies(env).unwrap_or("COPY . .".to_string()),
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

#[derive(Deserialize, Serialize, Default, PartialEq)]
pub struct ImagesReplacement {
  pub depl_from: String,
  pub depl_to: String,
  pub base_from: String,
  pub base_to: String,
}

pub static PREVENT_METADATA_LOCK: &str = ".deployer-prevent-metadata.lock.json";

pub fn check_and_pull_once(
  env: &RunEnvironment,
  opts: &mut ContaineredOpts,
  exclusive_exec_tag: &str,
  sudo: bool,
) -> anyhow::Result<()> {
  let mut repls = crate::rw::read::<ImagesReplacement>(env.run_dir, PREVENT_METADATA_LOCK);
  if repls.base_from.ne(opts.base_image.as_deref().unwrap_or(BASE_IMAGE)) {
    if !repls.base_to.is_empty()
      && run_simple(
        env,
        format!("{}docker rmi {}", if sudo { "" } else { "sudo " }, repls.base_to),
      )
      .is_err()
    {
      println!("{}", i18n::CTRD_CANT_REMOVE_OLD_IMG.red());
      exit(1);
    }
    if run_simple(
      env,
      format!(
        "{}docker pull {}",
        if sudo { "" } else { "sudo " },
        opts.base_image.as_deref().unwrap_or(BASE_IMAGE)
      ),
    )
    .is_err()
    {
      println!("{}", i18n::CTRD_CANT_PULL_IMG.red());
      exit(1);
    }
    if run_simple(
      env,
      format!(
        "{}docker tag {} {}_executor:latest",
        if sudo { "" } else { "sudo " },
        opts.base_image.as_deref().unwrap_or(BASE_IMAGE),
        exclusive_exec_tag
      ),
    )
    .is_err()
    {
      println!("{}", i18n::CTRD_CANT_TAG_IMG.red());
      exit(1);
    }
    repls.base_from = opts.base_image.as_deref().unwrap_or(BASE_IMAGE).to_owned();
    repls.base_to = format!("{}_executor:latest", exclusive_exec_tag);
  }
  if repls
    .depl_from
    .ne(opts.build_deployer_base_image.as_deref().unwrap_or(BASE_IMAGE))
  {
    if !repls.depl_to.is_empty()
      && run_simple(
        env,
        format!("{}docker rmi {}", if sudo { "" } else { "sudo " }, repls.depl_to),
      )
      .is_err()
    {
      println!("{}", i18n::CTRD_CANT_REMOVE_OLD_IMG.red());
      exit(1);
    }
    if run_simple(
      env,
      format!(
        "{}docker pull {}",
        if sudo { "" } else { "sudo " },
        opts.build_deployer_base_image.as_deref().unwrap_or(BASE_IMAGE)
      ),
    )
    .is_err()
    {
      println!("{}", i18n::CTRD_CANT_PULL_IMG.red());
      exit(1);
    }
    if run_simple(
      env,
      format!(
        "{}docker tag {} {}_builder:latest",
        if sudo { "" } else { "sudo " },
        opts.build_deployer_base_image.as_deref().unwrap_or(BASE_IMAGE),
        exclusive_exec_tag
      ),
    )
    .is_err()
    {
      println!("{}", i18n::CTRD_CANT_TAG_IMG.red());
      exit(1);
    }
    repls.depl_from = opts
      .build_deployer_base_image
      .as_deref()
      .unwrap_or(BASE_IMAGE)
      .to_owned();
    repls.depl_to = format!("{}_builder:latest", exclusive_exec_tag);
  }
  opts.base_image = Some(repls.base_to.to_owned());
  opts.build_deployer_base_image = Some(repls.depl_to.to_owned());
  crate::rw::write(env.run_dir, PREVENT_METADATA_LOCK, &repls);
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

  let sudo = is_user_in_group("docker")?;

  let exclusive_exec_tag = pipeline.exclusive_exec_tag.clone().unwrap_or(String::from("default")) + "-containered";
  let mut opts = pipeline.containered_opts.clone().unwrap();
  opts.sync_fake_content(env)?;
  if opts.prevent_metadata_loading.is_some_and(|v| v) {
    check_and_pull_once(env, &mut opts, &exclusive_exec_tag, sudo)?;
  }
  generate_dockerfile(env, pipeline, &opts, &exclusive_exec_tag)?;
  generate_dockerignore(env, config)?;
  println!(
    "{}",
    i18n::CTRD_START_BUILD.replace(
      "{}",
      &format!("{}", format!("{}/{}", config.project_name, pipeline.title).green())
    )
  );

  let build_cmd = format!(
    "{}docker build {}-t {}/{} -f Dockerfile.{}{} .",
    if sudo { "" } else { "sudo " },
    if env.new_build { "--no-cache " } else { "" },
    config.project_name,
    pipeline.title,
    exclusive_exec_tag,
    if opts.use_containerd_local_storage_cache.is_some_and(|v| v) {
      format!(
        " --cache-to type=local,dest=.docker-cache/{},compression=zstd --cache-from type=local,src=.docker-cache/{}",
        exclusive_exec_tag, exclusive_exec_tag
      )
    } else {
      String::from("")
    },
  );
  if run_simple(env, build_cmd.to_owned()).is_err() && run_simple(env, build_cmd).is_err() {
    println!("{}", i18n::CTRD_IMG_WASNT_BUILT.red());
    exit(1);
  }
  println!("{}", i18n::CTRD_IMG_WAS_BUILT.green());

  let volume_path = env.artifacts_dir.join(&pipeline.title);

  if run_simple(
    env,
    format!(
      "{}docker run -v {:?}:/app/artifacts {}/{}",
      if sudo { "" } else { "sudo " },
      volume_path,
      config.project_name,
      pipeline.title
    ),
  )
  .is_err()
  {
    println!("{}", i18n::CTRD_DEPL_WASNT_RAN.red());
    exit(1);
  }

  println!("{}", i18n::CTRD_DEPL_WAS_RAN.green());

  Ok(())
}
