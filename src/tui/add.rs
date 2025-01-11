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
use crate::configs::DeployerGlobalConfig;
use crate::entities::{
  auto_version::AutoVersionExtractFromRule,
  custom_command::{CustomCommand, specify_bash_c},
  info::{ActionInfo, ContentInfo, PipelineInfo},
  programming_languages::specify_programming_languages,
  targets::TargetDescription,
  variables::Variable,
};
use crate::hmap;
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::utils::tags_custom_type;

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
      "Check" => {
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
        
        Action::Check(CheckAction {
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
      },
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
    
    let described_action = DescribedAction {
      title: name,
      desc,
      info,
      tags,
      action,
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
