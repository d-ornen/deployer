use anyhow::bail;
use colored::Colorize;
use safe_path::scoped_join;
use std::path::PathBuf;

use crate::actions::{
  new_action,
  buildlike::*,
  check::{specify_regex, CheckAction},
  deploylike::*,
  observe::ObserveAction,
  packlike::*,
  patch::PatchAction,
  storage_add::AddToStorageAction,
  Action,
  DescribedAction,
};
use crate::cmd::NewActionArgs;
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::entities::{
  auto_version::AutoVersionExtractFromRule,
  custom_command::{CustomCommand, specify_bash_c},
  info::{ActionInfo, ContentInfo, PipelineInfo},
  programming_languages::{ProgrammingLanguage, specify_programming_languages},
  requirements::Requirement,
  targets::{TargetDescription, OsVariant, OsVersionSpecification},
  variables::{Variable, VarValue, FromEnvFile, Kv2Paths, VAULT_ADDR_ENV, VAULT_ADDR_TOKEN},
};
use crate::hmap;
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::utils::tags_custom_type;

impl DeployerProjectOptions {
  pub(crate) fn init_from_prompt(&mut self, curr_dir: String) -> anyhow::Result<()> {
    use inquire::Text;
    
    #[cfg(unix)]
    let curr_dir = curr_dir.split('/').next_back().unwrap();
    let project_name_proposal = if self.project_name.is_empty() {
      curr_dir.to_owned()
    } else {
      self.project_name.to_owned()
    };
    self.project_name = Text::new(i18n::PROJECT_NAME).with_initial_value(project_name_proposal.as_str()).prompt()?;
    
    self.cache_files.insert(PathBuf::from(".git"));
    println!("{}", i18n::PROJECT_SPECIFY_PLS);
    self.langs = specify_programming_languages()?;
    for lang in &self.langs {
      match lang {
        ProgrammingLanguage::Rust => self.cache_files.extend([PathBuf::from("Cargo.lock"), PathBuf::from("target")].into_iter()),
        ProgrammingLanguage::Go => self.cache_files.extend([PathBuf::from("go.sum"), PathBuf::from("vendor")].into_iter()),
        ProgrammingLanguage::Python => self.cache_files.extend([PathBuf::from("__pycache__"), PathBuf::from("dist")].into_iter()),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => self.cache_files.extend([PathBuf::from("CMakeFiles"), PathBuf::from("CMakeCache.txt")].into_iter()),
        _ => {},
      }
    }
    
    self.deploy_toolkit = Text::new(&format!("{} {}:", i18n::PROJECT_DEPL_TOOLKIT, i18n::OR_HIT_ESC)).prompt_skippable()?;
    self.targets = collect_targets()?;
    self.variables = collect_variables()?;
    self.artifacts = collect_artifacts()?;
    self.inplace_artifacts_into_project_root = collect_af_inplacements(&self.artifacts)?;
    
    Ok(())
  }
}

impl DescribedAction {
  pub(crate) fn new_from_prompt(opts: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::{Select, Text};
    
    let short_name = Text::new(i18n::ACTION_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::ACTION_VERSION).prompt()?;
    
    let info = ActionInfo::new(short_name, version)?;
    
    let name = Text::new(i18n::ACTION_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::ACTION_DESC).prompt()?;
    
    let tags: Vec<String> = tags_custom_type(i18n::ACTION_TAGS, None).prompt()?;
    
    let action_types: Vec<&str> = vec![
      "Interrupt",
      "Custom",
      "Check",
      "Force artifacts enplace",
      "Use content from storage",
      "Patch",
      "Pre-build",
      "Build",
      "Post-build",
      "Test",
      "Pack",
      "Deliver",
      "Install",
      "Configure deploy",
      "Deploy",
      "Post-deploy",
      "Observe",
      "Automatical push artifacts to the common storage",
    ];
    
    let selected_action_type = Select::new(i18n::ACTION_SELECT_TYPE, action_types).prompt()?;
    
    let action = match selected_action_type {
      "Interrupt" => Action::Interrupt,
      "Force artifacts enplace" => Action::ForceArtifactsEnplace,
      "Custom" => {
        let command = CustomCommand::new_from_prompt()?;
        Action::Custom(command)
      },
      "Check" => Action::Check(CheckAction::new_from_prompt()?),
      action_type @ ("Pre-build" | "Build" | "Post-build" | "Test") => {
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
          "Test" => Action::Test(action),
          _ => unreachable!(),
        }
      },
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
      },
      action_type @ ("Configure deploy" | "Deploy" | "Post-deploy") => {
        let deploy_toolkit = Text::new("Enter deploy toolkit name (or hit `esc`):").prompt_skippable()?;
        let tags = tags_custom_type("Enter deploy tags:", None).prompt()?;
        let commands = collect_multiple_commands()?;
        
        let action = DeployAction {
          deploy_toolkit,
          tags,
          commands,
        };
        
        match action_type {
          "Configure deploy" => Action::ConfigureDeploy(action),
          "Deploy" => Action::Deploy(action),
          "Post-deploy" => Action::PostDeploy(action),
          _ => unreachable!(),
        }
      },
      "Observe" => {
        let tags = tags_custom_type(i18n::OBSERVE_TAGS, None).prompt()?;
        let command = CustomCommand::new_from_prompt_unspecified()?;
        
        Action::Observe(ObserveAction { tags, command })
      },
      "Patch" => Action::Patch(PatchAction::new_from_prompt()?),
      "Use content from storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let version = Text::new(i18n::CONTENT_VER).prompt()?;
        
        let info = ContentInfo::new_for_using(short_name, version)?;
        
        Action::UseFromStorage(info)
      },
      "Automatical push artifacts to the common storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let auto_version_rule = AutoVersionExtractFromRule::new_from_prompt()?;
        
        Action::AddToStorage(AddToStorageAction { short_name, auto_version_rule })
      }
      _ => unreachable!(),
    };
    
    let requirements = collect_requirements()?;
    
    let described_action = DescribedAction {
      title: name,
      desc,
      info,
      tags,
      action,
      requirements,
    };
    
    if
      opts.actions_registry.contains_key(&described_action.info.to_str()) &&
      !inquire::Confirm::new(&i18n::ACTION_REG_ALREADY_HAVE.replace("{}", &described_action.info.to_str())).prompt()?
    {
      std::process::exit(0);
    }
    
    opts.actions_registry.insert(described_action.info.to_str(), described_action.clone());
    
    Ok(described_action)
  }
}

/// Создаёт несколько новых команд.
pub(crate) fn collect_multiple_commands() -> anyhow::Result<Vec<CustomCommand>> {
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

pub(crate) fn collect_requirements() -> anyhow::Result<Option<Vec<Requirement>>> {
  use inquire::Confirm;
  
  let mut reqs = Vec::new();
  while Confirm::new(i18n::ADD_REQ).with_default(false).prompt()? {
    if let Ok(req) = Requirement::new_from_prompt() { reqs.push(req); }
  }
  Ok(if reqs.is_empty() { None } else { Some(reqs) })
}

impl CheckAction {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let bash_c = specify_bash_c(None)?;
    
    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    let ignore_fails = !inquire::Confirm::new(i18n::CHECK_IGNORE_FAILS).with_default(true).prompt()?;
    
    let mut success_when_found = None;
    let mut success_when_not_found = None;
    loop {
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC).with_default(true).prompt()? {
        success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
      }
      
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL).with_default(true).prompt()? {
        success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
      }
      
      if success_when_found.is_some() || success_when_not_found.is_some() { break }
      else { println!("{}", i18n::CHECK_NEED_TO_AT_LEAST); }
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
      },
    })
  }
  
  pub(crate) fn new_wop_from_prompt() -> anyhow::Result<Self> {
    let bash_c = specify_bash_c(None)?;
    
    let ignore_fails = !inquire::Confirm::new(i18n::CHECK_IGNORE_FAILS).with_default(true).prompt()?;
    
    let mut success_when_found = None;
    let mut success_when_not_found = None;
    loop {
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC).with_default(true).prompt()? {
        success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
      }
      
      if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL).with_default(true).prompt()? {
        success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
      }
      
      if success_when_found.is_some() || success_when_not_found.is_some() { break }
      else { println!("{}", i18n::CHECK_NEED_TO_AT_LEAST); }
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
      },
    })
  }
}

impl DescribedPipeline {
  pub(crate) fn new_from_prompt(globals: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::Text;
    
    let short_name = Text::new(i18n::PIPELINE_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::PIPELINE_VERSION).prompt()?;
    
    let info = PipelineInfo::new(short_name, version)?;
    
    let name = Text::new(i18n::PIPELINE_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::PIPELINE_DESC).prompt()?;
    
    let tags: Vec<String> = tags_custom_type(i18n::PIPELINE_TAGS, None).prompt()?;
    
    let selected_actions_unordered = collect_multiple_actions(globals)?;
    let selected_actions_ordered = reorder_actions(selected_actions_unordered)?;
    
    let exclusive_exec_tag = Text::new(&format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)).prompt_skippable()?;
    
    let described_pipeline = DescribedPipeline {
      title: name,
      desc,
      info,
      tags,
      actions: selected_actions_ordered,
      default: None,
      exclusive_exec_tag,
    };
    
    Ok(described_pipeline)
  }
}

// Helper function to collect multiple custom commands
pub(crate) fn collect_multiple_actions(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<Vec<DescribedAction>> {
  use inquire::Confirm;
  
  let mut actions = Vec::new();
  let mut first = true;
  
  while Confirm::new(i18n::ACTION_ADD).with_default(first).prompt()? {
    actions.push(select_action(globals)?);
    first = false;
  }
  Ok(actions)
}

pub(crate) fn select_action(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<DescribedAction> {
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
    action.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, action.title, if action.desc.is_empty() { "" } else { " " }, action.desc);
    action.title = new_title;
    
    return Ok(action)
  }
  
  let mut action = (*actions.get(&selected_action).unwrap()).clone();
  
  let new_title = Text::new(i18n::PIPELINE_DESCRIBE_ACTION_IN).prompt()?;
  action.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, action.title, if action.desc.is_empty() { "" } else { " " }, action.desc);
  action.title = new_title;
  
  Ok(action)
}

pub(crate) fn reorder_actions(
  selected_actions_unordered: Vec<DescribedAction>,
) -> anyhow::Result<Vec<DescribedAction>> {
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

pub(crate) fn collect_targets() -> anyhow::Result<Vec<TargetDescription>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new(i18n::ADD_NEW_TARGET).with_default(first).prompt()? {
    v.push(TargetDescription::new_from_prompt()?);
    first = false;
  }
  
  Ok(v)
}

pub(crate) fn collect_artifact() -> anyhow::Result<PathBuf> {
  let assume_root = PathBuf::from("/");
  
  loop {
    let path = PathBuf::from(inquire::Text::new(i18n::RELATIVE_PATH).prompt()?);
    if scoped_join(&assume_root, &path).is_ok() { return Ok(path) }
    else { println!("{}", i18n::INCORRECT_PATH) }
  }
}

pub(crate) fn collect_artifacts() -> anyhow::Result<Vec<PathBuf>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new(i18n::ADD_NEW_AF).with_default(first).prompt()? {
    v.push(collect_artifact()?);
    first = false;
  }
  
  Ok(v)
}

pub(crate) fn collect_path() -> anyhow::Result<PathBuf> {
  Ok(PathBuf::from(inquire::Text::new(i18n::ABSOLUTE_PATH).prompt()?))
}

pub(crate) fn collect_paths() -> anyhow::Result<Vec<PathBuf>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new(i18n::ADD_NEW_PATH).with_default(first).prompt()? {
    v.push(collect_path()?);
    first = false;
  }
  
  Ok(v)
}

pub(crate) fn collect_variables() -> anyhow::Result<Vec<Variable>> {
  let mut v = vec![];
  let mut first = true;
  
  while inquire::Confirm::new(i18n::ADD_NEW_VAR).with_default(first).prompt()? {
    v.push(Variable::new_from_prompt()?);
    first = false;
  }
  
  Ok(v)
}

pub(crate) fn collect_af_inplacement(artifacts: &[impl AsRef<str>]) -> anyhow::Result<(PathBuf, PathBuf)> {
  use inquire::{Select, Text};
  
  let assume_root = PathBuf::from("/");
  let artifacts = artifacts.iter().map(|v| v.as_ref()).collect::<Vec<_>>();
  
  loop {
    let from = PathBuf::from(Select::new(i18n::SELECT_PROJECT_AF, artifacts.to_owned()).prompt()?);
    let to = PathBuf::from(Text::new(i18n::CHOOSE_AF_INPLACEMENT).prompt()?);
    if scoped_join(&assume_root, &to).is_ok() { return Ok((from, to)) }
    else { println!("{}", i18n::INCORRECT_AF_INPL_PATH) }
  }
}

pub(crate) fn collect_af_inplacements(artifacts: &[PathBuf]) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
  use inquire::Confirm;
  
  let artifacts = artifacts.iter().map(|v| v.to_str().unwrap()).collect::<Vec<_>>();
  
  const FIRST_PROMPT: &str = i18n::ADD_NEW_INPLACEMENT_FIRST;
  const ANOTHER_PROMPT: &str = i18n::ADD_NEW_INPLACEMENT_SECOND;
  
  let mut v = vec![];
  let mut prompt = FIRST_PROMPT;
  let mut first = true;
  
  if artifacts.is_empty() { first = false; }
  
  
  while Confirm::new(prompt).with_default(first).prompt()? {
    v.push(collect_af_inplacement(&artifacts)?);
    prompt = ANOTHER_PROMPT;
    first = false;
  }
  
  Ok(v)
}

impl Variable {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let title = inquire::Text::new(i18n::VAR_TITLE).prompt()?;
    println!("{}: {} `{}`, `{}`.", i18n::NOTE.green().italic(), i18n::VAR_NOTE, VAULT_ADDR_ENV.green(), VAULT_ADDR_TOKEN.green());
    let is_secret = inquire::Confirm::new(i18n::VAR_IS_SECRET).with_default(false).prompt()?;
    
    let types = vec![i18n::VAR_PLAIN, i18n::VAR_ENV, i18n::VAR_KV2];
    let r#type = inquire::Select::new(i18n::SPECIFY_VAR_TYPE, types).prompt()?;
    let value = match r#type {
      i18n::VAR_PLAIN => Variable::new_plain_from_prompt()?,
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
  
  pub(crate) fn new_plain_from_prompt() -> anyhow::Result<VarValue> {
    Ok(VarValue::Plain(inquire::Text::new(i18n::VAR_PLAIN_CONTENT).prompt()?))
  }
  
  pub(crate) fn new_env_from_prompt() -> anyhow::Result<VarValue> {
    Ok(VarValue::FromEnvFile(FromEnvFile {
      env_file_path: PathBuf::from(inquire::Text::new(i18n::VAR_ENV_FILE).prompt()?),
      key: inquire::Text::new(i18n::VAR_ENV_KEY).prompt()?,
    }))
  }
  
  pub(crate) fn new_kv2_from_prompt() -> anyhow::Result<VarValue> {
    println!("{}: {}", i18n::NOTE.green().italic(), i18n::KV2_NOTE);
    Ok(VarValue::FromHCVaultKv2(Kv2Paths {
      mount_path: inquire::Text::new(i18n::VAR_MOUNT_PATH).prompt()?,
      secret_path: inquire::Text::new(i18n::VAR_SECRET_PATH).prompt()?,
    }))
  }
}

impl Requirement {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let types = vec![i18n::REQ_TYPE_EX, i18n::REQ_TYPE_EX_ANY, i18n::REQ_TYPE_CHECK];
    let r#type = inquire::Select::new(i18n::SELECT_REQ_TYPE, types).prompt()?;
    match r#type {
      i18n::REQ_TYPE_EX => Requirement::new_exists_from_prompt(),
      i18n::REQ_TYPE_EX_ANY => Requirement::new_exists_any_from_prompt(),
      i18n::REQ_TYPE_CHECK => Requirement::new_check_from_prompt(),
      _ => unreachable!(),
    }
  }
  
  pub(crate) fn new_exists_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::Exists(collect_path()?))
  }
  
  pub(crate) fn new_exists_any_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::ExistsAny(collect_paths()?))
  }
  
  pub(crate) fn new_check_from_prompt() -> anyhow::Result<Self> {
    Ok(Self::CheckSuccess(CheckAction::new_wop_from_prompt()?))
  }
}

impl TargetDescription {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    use inquire::{Select, Text};
    
    let arch = Text::new(i18n::TARGET_ARCH).prompt()?;
    
    let os = Select::new(
      i18n::TARGET_OS_SELECT,
      vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
    ).prompt()?;
    
    let os_variant = match os {
      "Android" => OsVariant::Android,
      "iOS" => OsVariant::iOS,
      "Linux" => OsVariant::Linux,
      "Unix-like" => {
        let name = Text::new(i18n::TARGET_OS_UNIX_LIKE).prompt()?;
        OsVariant::UnixLike(name)
      },
      "Windows" => OsVariant::Windows,
      "macOS" => OsVariant::macOS,
      "Other" => {
        let name = Text::new(i18n::TARGET_OS_OTHER).prompt()?;
        OsVariant::Other(name)
      },
      _ => unreachable!(),
    };
    
    let derivative = Text::new(i18n::TARGET_OS_DER).prompt()?;
    
    let version_type = Select::new(
      i18n::TARGET_OS_VER_S,
      vec![i18n::TARGET_OS_VER_NS, i18n::TARGET_OS_VER_WS, i18n::TARGET_OS_VER_SS]
    ).prompt()?;
    
    let version = match version_type {
      i18n::TARGET_OS_VER_NS => OsVersionSpecification::No,
      i18n::TARGET_OS_VER_WS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
        OsVersionSpecification::Weak(ver)
      },
      i18n::TARGET_OS_VER_SS => {
        let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
        OsVersionSpecification::Strong(ver)
      },
      _ => unreachable!(),
    };
    
    Ok(TargetDescription {
      arch,
      os: os_variant,
      derivative,
      version,
    })
  }
}

impl AutoVersionExtractFromRule {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let new_autover_rule = inquire::Select::new(
      i18n::SPECIFY_AUTO_VER,
      vec![i18n::AUTO_VER_CMD_STDOUT, i18n::AUTO_VER_PLAIN_FILE],
    ).prompt()?;
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
  /// Создаёт новую команду.
  pub(crate) fn new_from_prompt() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;
    
    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    let ignore_fails = inquire::Confirm::new(i18n::CMD_IGNORE_FAILS).with_default(false).prompt()?;
    let show_bash_c = inquire::Confirm::new(i18n::CMD_SHOW_BASH_C).with_default(true).prompt()?;
    let show_success_output = inquire::Confirm::new(i18n::CMD_SHOW_SUCC_OUT).with_default(false).prompt()?;
    let only_when_fresh = Some(inquire::Confirm::new(i18n::CMD_ONLY_WHEN_FRESH).with_default(false).prompt()?);
    
    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails,
      show_bash_c,
      show_success_output,
      only_when_fresh,
      replacements: None,
    })
  }
  
  pub(crate) fn new_from_prompt_unspecified() -> anyhow::Result<CustomCommand> {
    let bash_c = specify_bash_c(None)?;
    
    let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
    let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
    
    Ok(CustomCommand {
      bash_c,
      placeholders,
      ignore_fails: true,
      show_success_output: true,
      show_bash_c: false,
      only_when_fresh: Some(false),
      replacements: None,
    })
  }
}

impl ProgrammingLanguage {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
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
