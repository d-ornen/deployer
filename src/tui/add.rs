//! `Add` and `New` menus.

use anyhow::bail;
use colored::Colorize;
use regex::Regex;
use safe_path::scoped_join;
use std::path::PathBuf;

use crate::actions::{
  Action, DescribedAction, buildlike::*, deploylike::*, new_action, observe::ObserveAction, packlike::*,
  patch::PatchAction, storage_add::AddToStorageAction, test::TestAction,
};
use crate::cmd::{NewActionArgs, NewPipelineArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions, Placement};
use crate::entities::{
  auto_version::AutoVersionExtractFromRule,
  custom_command::CustomCommand,
  info::{ActionInfo, ContentInfo, PipelineInfo, ShortName},
  programming_languages::ProgrammingLanguage,
  remote_host::RemoteHost,
  requirements::Requirement,
  targets::{OsVariant, OsVersionSpecification, TargetDescription},
  variables::{FromEnvFile, Kv2Paths, VAULT_ADDR_ENV, VAULT_ADDR_TOKEN, VarValue, Variable},
};
use crate::hmap;
use crate::i18n;
use crate::pipelines::{DescribedPipeline, new_pipeline};
use crate::utils::{str2regex_simple, tags_custom_type};

impl DeployerProjectOptions {
  pub fn init_from_prompt(&mut self, curr_dir: String) -> anyhow::Result<()> {
    use inquire::Text;

    #[cfg(unix)]
    let curr_dir = curr_dir.split('/').next_back().unwrap();
    let project_name_proposal = if self.project_name.is_empty() {
      curr_dir.to_owned()
    } else {
      self.project_name.to_owned()
    };
    self.project_name = Text::new(i18n::PROJECT_NAME)
      .with_initial_value(project_name_proposal.as_str())
      .prompt()?;

    self.cache_files.insert(PathBuf::from(".git"));
    println!("{}", i18n::PROJECT_SPECIFY_PLS);
    self.langs = specify_programming_languages()?;
    for lang in &self.langs {
      match lang {
        ProgrammingLanguage::Rust => self
          .cache_files
          .extend([PathBuf::from("Cargo.lock"), PathBuf::from("target")].into_iter()),
        ProgrammingLanguage::Go => self
          .cache_files
          .extend([PathBuf::from("go.sum"), PathBuf::from("vendor")].into_iter()),
        ProgrammingLanguage::Python => self
          .cache_files
          .extend([PathBuf::from("__pycache__"), PathBuf::from("dist")].into_iter()),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => self
          .cache_files
          .extend([PathBuf::from("CMakeFiles"), PathBuf::from("CMakeCache.txt")].into_iter()),
        _ => {}
      }
    }

    self.deploy_toolkit =
      Text::new(&format!("{} {}:", i18n::PROJECT_DEPL_TOOLKIT, i18n::OR_HIT_ESC)).prompt_skippable()?;
    self.targets = collect_targets()?;
    self.variables = collect_variables()?;
    self.artifacts = collect_artifacts()?;
    self.place_artifacts_into_project_root = collect_af_inplacements(&self.artifacts)?;

    Ok(())
  }
}

impl DescribedAction {
  pub fn new_from_prompt(opts: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::{Select, Text};

    let short_name = Text::new(i18n::ACTION_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::ACTION_VERSION).prompt()?;

    let info = ActionInfo::new(short_name, version)?;

    let name = Text::new(i18n::ACTION_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::ACTION_DESC).prompt()?;

    let tags: Vec<String> = tags_custom_type(i18n::ACTION_TAGS, None).prompt()?;

    let action_types: Vec<&str> = vec![
      "Custom",
      "Use content from storage",
      "Patch",
      "Sync build folder to remote",
      "Pre-build",
      "Build",
      "Post-build",
      "Sync build artifacts from remote",
      "Test",
      "Pack",
      "Deliver",
      "Install",
      "Configure deploy",
      "Deploy",
      "Post-deploy",
      "Observe",
      "Automatical push artifacts to the common storage",
      "Another Pipeline",
    ];

    let selected_action_type = Select::new(i18n::ACTION_SELECT_TYPE, action_types).prompt()?;

    let action = match selected_action_type {
      "Custom" => {
        let command = CustomCommand::new_from_prompt()?;
        Action::Custom(command)
      }
      "Test" => Action::Test(TestAction::new_from_prompt()?),
      action_type @ ("Pre-build" | "Build" | "Post-build") => {
        let supported_langs = specify_programming_languages()?;
        let commands = collect_multiple_commands()?;

        let action = BuildAction {
          supported_langs,
          commands,
        };

        match action_type {
          "Pre-build" => Action::PreBuild(action),
          "Build" => Action::Build(action),
          "Post-build" => Action::PostBuild(action),
          _ => unreachable!(),
        }
      }
      action_type @ ("Pack" | "Deliver" | "Install") => {
        let target = TargetDescription::new_from_prompt()?;
        let commands = collect_multiple_commands()?;

        let action = PackAction {
          target: Some(target),
          commands,
        };

        match action_type {
          "Pack" => Action::Pack(action),
          "Deliver" => Action::Deliver(action),
          "Install" => Action::Install(action),
          _ => unreachable!(),
        }
      }
      action_type @ ("Configure deploy" | "Deploy" | "Post-deploy") => {
        let deploy_toolkit = Text::new("Enter deploy toolkit name (or hit `esc`):").prompt_skippable()?;
        let commands = collect_multiple_commands()?;

        let action = DeployAction {
          deploy_toolkit,
          commands,
        };

        match action_type {
          "Configure deploy" => Action::ConfigureDeploy(action),
          "Deploy" => Action::Deploy(action),
          "Post-deploy" => Action::PostDeploy(action),
          _ => unreachable!(),
        }
      }
      "Observe" => Action::Observe(ObserveAction {
        command: CustomCommand::new_from_prompt_unspecified()?,
      }),
      "Patch" => Action::Patch(PatchAction::new_from_prompt()?),
      "Use content from storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let version = Text::new(i18n::CONTENT_VER).prompt()?;

        let info = ContentInfo::new_for_using(short_name, version)?;

        Action::UseFromStorage { content_info: info }
      }
      "Automatical push artifacts to the common storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let auto_version_rule = AutoVersionExtractFromRule::new_from_prompt()?;

        Action::AddToStorage(AddToStorageAction {
          short_name,
          auto_version_rule,
        })
      }
      "Sync build folder to remote" => Action::SyncToRemote {
        remote_host_name: ShortName::new(inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?)?,
      },
      "Sync build artifacts from remote" => Action::SyncFromRemote {
        remote_host_name: ShortName::new(inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?)?,
      },
      "Another Pipeline" => Action::SubPipeline(Box::new(select_pipeline(opts)?)),
      _ => unreachable!(),
    };

    let requirements = collect_requirements()?;

    let exec_in_project_dir = if let Ok(Some(exec_in_project_dir)) = inquire::Confirm::new(i18n::EXEC_IN_PROJECT_DIR)
      .with_default(false)
      .prompt_skippable()
    {
      Some(exec_in_project_dir)
    } else {
      None
    };

    let described_action = DescribedAction {
      title: name,
      desc,
      info,
      tags,
      action,
      requirements,
      exec_in_project_dir,
    };

    if opts.actions_registry.contains_key(&described_action.info)
      && !inquire::Confirm::new(&i18n::ACTION_REG_ALREADY_HAVE.replace("{}", &described_action.info.to_str()))
        .prompt()?
    {
      std::process::exit(0);
    }

    opts
      .actions_registry
      .insert(described_action.info.clone(), described_action.clone());

    Ok(described_action)
  }
}

pub fn collect_multiple_commands() -> anyhow::Result<Vec<CustomCommand>> {
  use inquire::Confirm;

  let mut commands = Vec::new();
  let mut first = true;
  while Confirm::new(i18n::ADD_CMD).with_default(first).prompt()? {
    if let Ok(command) = CustomCommand::new_from_prompt() {
      commands.push(command);
    }
    first = false;
  }
  Ok(commands)
}

pub fn collect_requirements() -> anyhow::Result<Option<Vec<Requirement>>> {
  use inquire::Confirm;

  let mut reqs = Vec::new();
  while Confirm::new(i18n::ADD_REQ).with_default(false).prompt()? {
    if let Ok(req) = Requirement::new_from_prompt() {
      reqs.push(req);
    }
  }
  Ok(if reqs.is_empty() { None } else { Some(reqs) })
}

impl TestAction {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let bash_c = specify_bash_c(None)?;

    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() {
      None
    } else {
      Some(placeholders)
    };

    let ignore_fails = !inquire::Confirm::new(i18n::CHECK_IGNORE_FAILS)
      .with_default(true)
      .prompt()?;

    let mut success_when_found = None;
    let mut success_when_not_found = None;
    loop {
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC)
        .with_default(true)
        .prompt()?
      {
        success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
      }

      if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL)
        .with_default(true)
        .prompt()?
      {
        success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
      }

      if success_when_found.is_some() || success_when_not_found.is_some() {
        break;
      } else {
        println!("{}", i18n::CHECK_NEED_TO_AT_LEAST);
      }
    }

    Ok(Self {
      success_when_found,
      success_when_not_found,
      command: CustomCommand {
        bash_c,
        placeholders,
        replacements: None,
        ignore_fails,
        show_success_output: true,
        show_bash_c: false,
        only_when_fresh: None,
        remote_exec: None,
      },
    })
  }

  pub fn new_wop_from_prompt() -> anyhow::Result<Self> {
    let bash_c = specify_bash_c(None)?;

    let ignore_fails = !inquire::Confirm::new(i18n::CHECK_IGNORE_FAILS)
      .with_default(true)
      .prompt()?;

    let mut success_when_found = None;
    let mut success_when_not_found = None;
    loop {
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC)
        .with_default(true)
        .prompt()?
      {
        success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
      }

      if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL)
        .with_default(true)
        .prompt()?
      {
        success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
      }

      if success_when_found.is_some() || success_when_not_found.is_some() {
        break;
      } else {
        println!("{}", i18n::CHECK_NEED_TO_AT_LEAST);
      }
    }

    Ok(Self {
      success_when_found,
      success_when_not_found,
      command: CustomCommand {
        bash_c,
        placeholders: None,
        replacements: None,
        ignore_fails,
        show_success_output: true,
        show_bash_c: false,
        only_when_fresh: None,
        remote_exec: None,
      },
    })
  }
}

impl PatchAction {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let patch = PathBuf::from(inquire::Text::new(i18n::PATCH_SPECIFY_PATH).prompt()?);
    Ok(Self { patch })
  }
}

impl DescribedPipeline {
  pub fn new_from_prompt(globals: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::Text;

    let short_name = Text::new(i18n::PIPELINE_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::PIPELINE_VERSION).prompt()?;

    let info = PipelineInfo::new(short_name, version)?;

    let name = Text::new(i18n::PIPELINE_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::PIPELINE_DESC).prompt()?;

    let tags: Vec<String> = tags_custom_type(i18n::PIPELINE_TAGS, None).prompt()?;

    let selected_actions_unordered = collect_multiple_actions(globals)?;
    let selected_actions_ordered = reorder_actions(selected_actions_unordered)?;

    let exclusive_exec_tag =
      Text::new(&format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)).prompt_skippable()?;

    let described_pipeline = DescribedPipeline {
      title: name,
      desc,
      info,
      tags,
      actions: selected_actions_ordered,
      default: None,
      exclusive_exec_tag,
      containered_opts: None,
    };

    Ok(described_pipeline)
  }
}

// Helper function to collect multiple Actions
pub fn collect_multiple_actions(globals: &mut DeployerGlobalConfig) -> anyhow::Result<Vec<DescribedAction>> {
  use inquire::Confirm;

  let mut actions = Vec::new();
  let mut first = true;

  while Confirm::new(i18n::ACTION_ADD).with_default(first).prompt()? {
    actions.push(select_action(globals)?);
    first = false;
  }
  Ok(actions)
}

pub fn select_action(globals: &mut DeployerGlobalConfig) -> anyhow::Result<DescribedAction> {
  use inquire::{Select, Text};

  let (actions, keys) = {
    let mut h = hmap!();
    let mut k = vec![];

    for key in globals.actions_registry.keys() {
      let action = globals.actions_registry.get(key).unwrap();
      let new_key = format!("{} - {}", action.info.to_str(), action.title);
      h.insert(new_key.clone(), action);
      k.push(new_key);
    }

    k.sort();
    k.push(i18n::ACTION_SPECIFY_ANOTHER.to_string());

    (h, k)
  };

  let selected_action = Select::new(i18n::SELECT_ACTION_TO_ADD_TO, keys).prompt()?;

  if selected_action.as_str().eq(i18n::ACTION_SPECIFY_ANOTHER) {
    let mut action = new_action(globals, &NewActionArgs { from: None })?;
    let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
    action.desc = format!(
      r#"{} `{}`.{}{}"#,
      i18n::GOT_FROM,
      action.title,
      if action.desc.is_empty() { "" } else { " " },
      action.desc
    );
    action.title = new_title;

    return Ok(action);
  }

  let mut action = (*actions.get(&selected_action).unwrap()).clone();

  let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
  action.desc = format!(
    r#"{} `{}`.{}{}"#,
    i18n::GOT_FROM,
    action.title,
    if action.desc.is_empty() { "" } else { " " },
    action.desc
  );
  action.title = new_title;

  Ok(action)
}

pub fn select_pipeline(globals: &mut DeployerGlobalConfig) -> anyhow::Result<DescribedPipeline> {
  use inquire::{Select, Text};

  let (actions, keys) = {
    let mut h = hmap!();
    let mut k = vec![];

    for key in globals.pipelines_registry.keys() {
      let pipeline = globals.pipelines_registry.get(key).unwrap();
      let new_key = format!("{} - {}", pipeline.info.to_str(), pipeline.title);
      h.insert(new_key.clone(), pipeline);
      k.push(new_key);
    }

    k.sort();
    k.push(i18n::PIPELINE_SPECIFY_ANOTHER.to_string());

    (h, k)
  };

  let selected_action = Select::new(i18n::SELECT_PIPELINE_TO_ADD_TO, keys).prompt()?;

  if selected_action.as_str().eq(i18n::PIPELINE_SPECIFY_ANOTHER) {
    let mut pipeline = new_pipeline(globals, &NewPipelineArgs { from: None })?;
    let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
    pipeline.desc = format!(
      r#"{} `{}`.{}{}"#,
      i18n::GOT_FROM,
      pipeline.title,
      if pipeline.desc.is_empty() { "" } else { " " },
      pipeline.desc
    );
    pipeline.title = new_title;

    return Ok(pipeline);
  }

  let mut action = (*actions.get(&selected_action).unwrap()).clone();

  let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
  action.desc = format!(
    r#"{} `{}`.{}{}"#,
    i18n::GOT_FROM,
    action.title,
    if action.desc.is_empty() { "" } else { " " },
    action.desc
  );
  action.title = new_title;

  Ok(action)
}

pub fn reorder_actions(selected_actions_unordered: Vec<DescribedAction>) -> anyhow::Result<Vec<DescribedAction>> {
  use inquire::ReorderableList;

  let mut h = hmap!();
  let mut k = vec![];

  for selected_action in selected_actions_unordered {
    let key = format!("{} - {}", selected_action.info.to_str(), selected_action.title);
    k.push(key.clone());
    h.insert(key, selected_action);
  }

  let reordered = ReorderableList::new(i18n::REORDER_PIPELINE_ACTIONS, k).prompt()?;

  let mut selected_actions_ordered = vec![];
  for key in reordered {
    selected_actions_ordered.push((*h.get(&key).unwrap()).clone());
  }

  Ok(selected_actions_ordered)
}

pub fn collect_targets() -> anyhow::Result<Vec<TargetDescription>> {
  let mut v = vec![];
  let mut first = true;

  while inquire::Confirm::new(i18n::ADD_NEW_TARGET)
    .with_default(first)
    .prompt()?
  {
    v.push(TargetDescription::new_from_prompt()?);
    first = false;
  }

  Ok(v)
}

pub fn collect_artifact() -> anyhow::Result<PathBuf> {
  let assume_root = PathBuf::from("/");

  loop {
    let path = PathBuf::from(inquire::Text::new(i18n::RELATIVE_PATH).prompt()?);
    if scoped_join(&assume_root, &path).is_ok() {
      return Ok(path);
    } else {
      println!("{}", i18n::INCORRECT_PATH)
    }
  }
}

pub fn collect_artifacts() -> anyhow::Result<Vec<PathBuf>> {
  let mut v = vec![];
  let mut first = true;

  while inquire::Confirm::new(i18n::ADD_NEW_AF).with_default(first).prompt()? {
    v.push(collect_artifact()?);
    first = false;
  }

  Ok(v)
}

pub fn collect_path() -> anyhow::Result<PathBuf> {
  Ok(PathBuf::from(inquire::Text::new(i18n::ABSOLUTE_PATH).prompt()?))
}

pub fn collect_paths() -> anyhow::Result<Vec<PathBuf>> {
  let mut v = vec![];
  let mut first = true;

  while inquire::Confirm::new(i18n::ADD_NEW_PATH).with_default(first).prompt()? {
    v.push(collect_path()?);
    first = false;
  }

  Ok(v)
}

pub fn collect_variables() -> anyhow::Result<Vec<Variable>> {
  let mut v = vec![];
  let mut first = true;

  while inquire::Confirm::new(i18n::ADD_NEW_VAR).with_default(first).prompt()? {
    v.push(Variable::new_from_prompt()?);
    first = false;
  }

  Ok(v)
}

pub fn collect_af_inplacement(artifacts: &[impl AsRef<str>]) -> anyhow::Result<Placement> {
  use inquire::{Select, Text};

  let assume_root = PathBuf::from("/");
  let artifacts = artifacts.iter().map(|v| v.as_ref()).collect::<Vec<_>>();

  loop {
    let from = PathBuf::from(Select::new(i18n::SELECT_PROJECT_AF, artifacts.to_owned()).prompt()?);
    let to = PathBuf::from(Text::new(i18n::CHOOSE_AF_INPLACEMENT).prompt()?);
    if scoped_join(&assume_root, &to).is_ok() {
      return Ok(Placement { from, to });
    } else {
      println!("{}", i18n::INCORRECT_AF_INPL_PATH)
    }
  }
}

pub fn collect_af_inplacements(artifacts: &[PathBuf]) -> anyhow::Result<Vec<Placement>> {
  use inquire::Confirm;

  let artifacts = artifacts.iter().map(|v| v.to_str().unwrap()).collect::<Vec<_>>();

  const FIRST_PROMPT: &str = i18n::ADD_NEW_INPLACEMENT_FIRST;
  const ANOTHER_PROMPT: &str = i18n::ADD_NEW_INPLACEMENT_SECOND;

  let mut v = vec![];
  let mut prompt = FIRST_PROMPT;
  let mut first = true;

  if artifacts.is_empty() {
    first = false;
  }

  while Confirm::new(prompt).with_default(first).prompt()? {
    v.push(collect_af_inplacement(&artifacts)?);
    prompt = ANOTHER_PROMPT;
    first = false;
  }

  Ok(v)
}

pub fn specify_regex(for_what: &str) -> anyhow::Result<Regex> {
  let mut regex_str;

  loop {
    regex_str = inquire::Text::new(&format!(
      "{} {} {}:",
      i18n::CHECK_ENTER_REGEX,
      for_what,
      i18n::CHECK_HELP
    ))
    .prompt()?;

    if let Err(e) = Regex::new(&regex_str) {
      println!("{}: {:?}.", i18n::CHECK_REGEX_INVALID_DUE, e);
      continue;
    }

    if regex_str.as_str() != "/h" {
      break;
    }
    println!("{}: `{}`", i18n::GUIDE, i18n::CHECK_GUIDE_TITLE.blue());
    println!(">>> {}", i18n::CHECK_GUIDE_1);
    println!(">>> {}", i18n::CHECK_GUIDE_2);
    println!(">>> ");
    println!(
      ">>> {}: {}",
      i18n::CHECK_GUIDE_3,
      "https://docs.rs/regex/latest/regex/".blue()
    );
    println!(
      ">>> {}: {} ({})",
      i18n::CHECK_GUIDE_4,
      "https://regex101.com/".blue(),
      i18n::CHECK_GUIDE_5
    );
  }

  str2regex_simple(regex_str.as_str())
}

impl Variable {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let title = inquire::Text::new(i18n::VAR_TITLE).prompt()?;
    println!(
      "{}: {} `{}`, `{}`.",
      i18n::NOTE.green().italic(),
      i18n::VAR_NOTE,
      VAULT_ADDR_ENV.green(),
      VAULT_ADDR_TOKEN.green()
    );
    let is_secret = inquire::Confirm::new(i18n::VAR_IS_SECRET)
      .with_default(false)
      .prompt()?;

    let types = vec![i18n::VAR_PLAIN, i18n::VAR_ENVF, i18n::VAR_ENV, i18n::VAR_KV2];
    let r#type = inquire::Select::new(i18n::SPECIFY_VAR_TYPE, types).prompt()?;
    let value = match r#type {
      i18n::VAR_PLAIN => Variable::new_plain_from_prompt()?,
      i18n::VAR_ENVF => Variable::new_env_file_from_prompt()?,
      i18n::VAR_ENV => Variable::new_env_from_prompt()?,
      i18n::VAR_KV2 => Variable::new_kv2_from_prompt()?,
      _ => unreachable!(),
    };

    Ok(Variable {
      title,
      is_secret,
      value,
    })
  }

  pub fn new_plain_from_prompt() -> anyhow::Result<VarValue> {
    Ok(VarValue::Plain {
      value: inquire::Text::new(i18n::VAR_PLAIN_CONTENT).prompt()?,
    })
  }

  pub fn new_env_from_prompt() -> anyhow::Result<VarValue> {
    Ok(VarValue::FromEnvVar {
      var_name: inquire::Text::new(i18n::VAR_ENV_KEY).prompt()?,
    })
  }

  pub fn new_env_file_from_prompt() -> anyhow::Result<VarValue> {
    Ok(VarValue::FromEnvFile(FromEnvFile {
      env_file_path: PathBuf::from(inquire::Text::new(i18n::VAR_ENV_FILE).prompt()?),
      key: inquire::Text::new(i18n::VAR_ENV_KEY).prompt()?,
    }))
  }

  pub fn new_kv2_from_prompt() -> anyhow::Result<VarValue> {
    println!("{}: {}", i18n::NOTE.green().italic(), i18n::KV2_NOTE);
    Ok(VarValue::FromHcVaultKv2(Kv2Paths {
      mount_path: inquire::Text::new(i18n::VAR_MOUNT_PATH).prompt()?,
      secret_path: inquire::Text::new(i18n::VAR_SECRET_PATH).prompt()?,
    }))
  }
}

impl Requirement {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let types = vec![
      i18n::REQ_TYPE_EX,
      i18n::REQ_TYPE_EX_ANY,
      i18n::REQ_TYPE_CHECK,
      i18n::REQ_TYPE_REMOTE,
    ];
    let r#type = inquire::Select::new(i18n::SELECT_REQ_TYPE, types).prompt()?;
    match r#type {
      i18n::REQ_TYPE_EX => Requirement::new_exists_from_prompt(),
      i18n::REQ_TYPE_EX_ANY => Requirement::new_exists_any_from_prompt(),
      i18n::REQ_TYPE_CHECK => Requirement::new_check_from_prompt(),
      i18n::REQ_TYPE_REMOTE => Requirement::new_remote_from_prompt(),
      _ => unreachable!(),
    }
  }

  pub fn new_exists_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::Exists { path: collect_path()? })
  }

  pub fn new_exists_any_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::ExistsAny {
      paths: collect_paths()?,
    })
  }

  pub fn new_check_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::CheckSuccess {
      action: TestAction::new_wop_from_prompt()?,
    })
  }

  pub fn new_remote_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::RemoteAccessibleAndReady {
      remote_host_name: ShortName::new(inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?)?,
    })
  }
}

impl TargetDescription {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    use inquire::{Select, Text};

    let arch = Text::new(i18n::TARGET_ARCH).prompt()?;

    let os = Select::new(
      i18n::TARGET_OS_SELECT,
      vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"],
    )
    .prompt()?;

    let os_variant = match os {
      "Android" => OsVariant::Android,
      "iOS" => OsVariant::iOS,
      "Linux" => OsVariant::Linux,
      "Unix-like" => OsVariant::UnixLike,
      "Windows" => OsVariant::Windows,
      "macOS" => OsVariant::macOS,
      "Other" => {
        let name = Text::new(i18n::TARGET_OS_OTHER).prompt()?;
        OsVariant::Other(name)
      }
      _ => unreachable!(),
    };

    let os_derivative = Text::new(i18n::TARGET_OS_DER).prompt()?;

    let version_type = Select::new(
      i18n::TARGET_OS_VER_S,
      vec![i18n::TARGET_OS_VER_NS, i18n::TARGET_OS_VER_WS, i18n::TARGET_OS_VER_SS],
    )
    .prompt()?;

    let os_version = match version_type {
      i18n::TARGET_OS_VER_NS => OsVersionSpecification::No,
      i18n::TARGET_OS_VER_WS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
        OsVersionSpecification::Weak { version: ver }
      }
      i18n::TARGET_OS_VER_SS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
        OsVersionSpecification::Strong { version: ver }
      }
      _ => unreachable!(),
    };

    Ok(TargetDescription {
      arch,
      os: os_variant,
      os_derivative,
      os_version,
    })
  }
}

impl AutoVersionExtractFromRule {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let new_autover_rule = inquire::Select::new(
      i18n::SPECIFY_AUTO_VER,
      vec![i18n::AUTO_VER_CMD_STDOUT, i18n::AUTO_VER_PLAIN_FILE],
    )
    .prompt()?;
    let auto_version_rule = match new_autover_rule {
      i18n::AUTO_VER_CMD_STDOUT => AutoVersionExtractFromRule::CmdStdout({
        let mut cmd = CustomCommand::new_from_prompt_unspecified()?;
        cmd.show_success_output = true;
        cmd
      }),
      i18n::AUTO_VER_PLAIN_FILE => AutoVersionExtractFromRule::PlainFile({
        let path = inquire::Text::new(i18n::SPECIFY_AUTO_VER_RELATIVE_FILEPATH).prompt()?;
        PathBuf::from(path)
      }),
      _ => bail!("There is no such type"),
    };
    Ok(auto_version_rule)
  }
}

impl CustomCommand {
  pub fn new_from_prompt() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;

    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() {
      None
    } else {
      Some(placeholders)
    };

    let ignore_fails = inquire::Confirm::new(i18n::CMD_IGNORE_FAILS)
      .with_default(false)
      .prompt()?;
    let show_bash_c = inquire::Confirm::new(i18n::CMD_SHOW_BASH_C)
      .with_default(true)
      .prompt()?;
    let show_success_output = inquire::Confirm::new(i18n::CMD_SHOW_SUCC_OUT)
      .with_default(false)
      .prompt()?;
    let only_when_fresh = Some(
      inquire::Confirm::new(i18n::CMD_ONLY_WHEN_FRESH)
        .with_default(false)
        .prompt()?,
    );

    let remote_exec = collect_remote()?;

    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails,
      show_bash_c,
      show_success_output,
      only_when_fresh,
      replacements: None,
      remote_exec,
    })
  }

  pub fn new_from_prompt_unspecified() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;

    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() {
      None
    } else {
      Some(placeholders)
    };

    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails: true,
      show_success_output: true,
      show_bash_c: false,
      only_when_fresh: Some(false),
      replacements: None,
      remote_exec: None,
    })
  }
}

pub fn specify_bash_c(default: Option<&str>) -> anyhow::Result<String> {
  let mut bash_c;
  loop {
    let prompt = format!("{} {}:", i18n::CMD_SPECIFY_BASH_C, i18n::CHECK_HELP);
    let mut text_prompt = inquire::Text::new(prompt.as_str());
    if let Some(default) = default {
      text_prompt = text_prompt.with_initial_value(default);
    }
    bash_c = text_prompt.prompt()?;
    if bash_c.as_str() != "/h" {
      break;
    }
    println!("{}: `{}`", i18n::GUIDE, i18n::CUSTOM_CMD_GUIDE_TITLE.blue());
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_1);
    println!(
      ">>> {}",
      i18n::CUSTOM_CMD_GUIDE_2
        .replace("%1%", &"~".green())
        .replace("%2%", &"PATH".green())
    );
    println!(">>> ");
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_3);
    println!(">>> `{}`", "g++ <input-file> -o <output-file>".green());
    println!(
      ">>> `{}{}`",
      "docker compose run -e DEPLOY_KEY=".green(),
      "{{my very secret key}}".red()
    );
    println!(">>> ");
    println!(">>> {}", i18n::CUSTOM_CMD_GUIDE_4);
    println!(">>> {} `{}`.", i18n::CUSTOM_CMD_GUIDE_5, "/bin/bash".green());

    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => format!("`{}`", path.green()),
      Err(_) => format!("\"\" (`{}`)", "/bin/bash".green()),
    };

    println!(">>> {} {}", i18n::CUSTOM_CMD_GUIDE_6, shell);
  }

  Ok(bash_c)
}

impl ProgrammingLanguage {
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let s = inquire::Text::new(i18n::PL_INPUT_PROMPT).prompt()?;
    let pl = match s.as_str() {
      "Rust" => Self::Rust,
      "Go" => Self::Go,
      "C" => Self::C,
      "C++" => Self::Cpp,
      "Python" => Self::Python,
      s => Self::Other(s.to_owned()),
    };
    Ok(pl)
  }
}

/// Parses specified programming languages.
pub fn specify_programming_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  use inquire::MultiSelect;

  let langs = vec!["Rust", "Go", "C", "C++", "Python", "Others"];
  let selected = MultiSelect::new(i18n::PL_SELECT, langs).prompt()?;

  let mut result = Vec::new();
  for lang in selected {
    let lang = match lang {
      "Rust" => ProgrammingLanguage::Rust,
      "Go" => ProgrammingLanguage::Go,
      "C" => ProgrammingLanguage::C,
      "C++" => ProgrammingLanguage::Cpp,
      "Python" => ProgrammingLanguage::Python,
      "Others" => {
        let langs = collect_multiple_languages()?;
        result.extend_from_slice(&langs);
        continue;
      }
      _ => unreachable!(),
    };
    result.push(lang);
  }

  Ok(result)
}

/// Collects multiple languages.
fn collect_multiple_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  let langs = tags_custom_type(i18n::PL_COLLECT, None).prompt()?;
  let mut v = vec![];

  for lang in langs {
    match lang.as_str() {
      "Rust" | "Go" | "C" | "C++" | "Python" => continue,
      lang => v.push(ProgrammingLanguage::Other(lang.to_owned())),
    }
  }

  Ok(v)
}

impl RemoteHost {
  #[allow(unused)]
  pub fn new_from_prompt() -> anyhow::Result<Self> {
    let short_name = ShortName::new(inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?)?;
    let ip = inquire::Text::new(i18n::SPECIFY_HOST_IP).prompt()?.parse()?;
    let port = inquire::Text::new(i18n::SPECIFY_HOST_PORT).prompt()?.parse()?;
    let username = inquire::Text::new(i18n::SPECIFY_HOST_USERNAME).prompt()?;
    let ssh_key_path = PathBuf::from(inquire::Text::new(i18n::SPECIFY_SSH_KEY_PATH).prompt()?);
    if !ssh_key_path.exists() {
      bail!("Specified private SSH key doesn't exists.")
    }

    Ok(Self {
      short_name,
      ip,
      port,
      username,
      ssh_private_key_file: ssh_key_path,
    })
  }

  pub fn new_with_args_from_prompt(args: crate::cmd::NewRemoteArgs) -> anyhow::Result<Self> {
    let short_name = ShortName::new(match args.short_name {
      Some(name) => name,
      None => inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?,
    })?;
    let ip = match args.ip {
      Some(ip) => ip,
      None => inquire::Text::new(i18n::SPECIFY_HOST_IP).prompt()?.parse()?,
    };
    let port = match args.port {
      Some(port) => port,
      None => inquire::Text::new(i18n::SPECIFY_HOST_PORT).prompt()?.parse()?,
    };
    let username = match args.username {
      Some(username) => username,
      None => inquire::Text::new(i18n::SPECIFY_HOST_USERNAME).prompt()?,
    };
    let ssh_key_path = match args.ssh_key_path {
      Some(path) => path,
      None => PathBuf::from(inquire::Text::new(i18n::SPECIFY_SSH_KEY_PATH).prompt()?),
    };
    if !ssh_key_path.exists() {
      bail!("Specified private SSH key doesn't exists.")
    }

    Ok(Self {
      short_name,
      ip,
      port,
      username,
      ssh_private_key_file: ssh_key_path,
    })
  }
}

fn collect_remote() -> anyhow::Result<Option<Vec<ShortName>>> {
  const PROMPT: &str = i18n::REMOTE_ADD_TO_CMD_FIRST;
  const ANOTHER_PROMPT: &str = i18n::REMOTE_ADD_TO_CMD_ANOTHER;

  let mut v = vec![];
  let mut prompt = PROMPT;

  while inquire::Confirm::new(prompt).with_default(false).prompt()? {
    v.push(ShortName::new(inquire::Text::new(i18n::REMOTE_SHORT_NAME).prompt()?)?);
    prompt = ANOTHER_PROMPT;
  }

  Ok(if !v.is_empty() { Some(v) } else { None })
}
