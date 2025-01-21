//! `Setup for project` menus.

use colored::Colorize;
use std::path::PathBuf;

use crate::actions::{Action, DescribedAction};
use crate::actions::{
  buildlike::BuildAction, check::CheckAction, deploylike::DeployAction, observe::ObserveAction, packlike::PackAction,
};
use crate::configs::DeployerProjectOptions;
use crate::entities::custom_command::CustomCommand;
use crate::entities::info::ActionInfo;
use crate::entities::programming_languages::ProgrammingLanguage;
use crate::entities::targets::TargetDescription;
use crate::entities::variables::{VarTraits, Variable};
use crate::i18n;

impl DescribedAction {
  fn setup_buildlike_action(
    &self,
    action: &BuildAction,
    langs: &Vec<ProgrammingLanguage>,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<BuildAction> {
    let mut action = action.clone();
    if !langs.is_empty()
      && !langs.iter().any(|l| action.supported_langs.contains(l))
      && !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_PLS
          .replace("{1}", &self.info.to_str())
          .replace("{2}", &format!("{:?}", action.supported_langs))
          .replace("{3}", &format!("{:?}", langs)),
      )
      .prompt()?
    {
      return Ok(BuildAction::default());
    }

    for cmd in &mut action.commands {
      *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?;
    }

    Ok(action)
  }

  fn setup_packlike_action(
    &self,
    action: &PackAction,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<PackAction> {
    let mut action = action.clone();

    if action.target.as_ref().is_some_and(|t| !targets.contains(t))
      && !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_TARGETS
          .replace("{1}", &self.info.to_str())
          .replace("{2}", &format!("{}", action.target.as_ref().unwrap()))
          .replace(
            "{3}",
            &format!(
              "{:?}",
              targets.iter().map(TargetDescription::to_string).collect::<Vec<_>>()
            ),
          ),
      )
      .prompt()?
    {
      return Ok(PackAction::default());
    }

    for cmd in &mut action.commands {
      *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?;
    }
    Ok(action)
  }

  fn setup_deploylike_action(
    &self,
    action: &DeployAction,
    deploy_toolkit: &Option<String>,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<DeployAction> {
    let mut action = action.clone();
    if action
      .deploy_toolkit
      .as_ref()
      .is_some_and(|l| deploy_toolkit.as_ref().is_some_and(|r| l.as_str() != r.as_str()))
      && !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_DEPL_TOOLKIT
          .replace("{1}", &self.info.to_str())
          .replace("{2}", action.deploy_toolkit.as_ref().unwrap())
          .replace("{3}", deploy_toolkit.as_ref().unwrap()),
      )
      .prompt()?
    {
      return Ok(DeployAction::default());
    }

    for cmd in &mut action.commands {
      *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?;
    }

    Ok(action)
  }

  fn setup_observe_action(
    &self,
    action: &ObserveAction,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<ObserveAction> {
    let mut action = action.clone();

    action.command = action
      .command
      .prompt_setup_for_project(&self.info, variables, artifacts)?;

    Ok(action)
  }

  pub fn prompt_setup_for_project(
    &self,
    langs: &Vec<ProgrammingLanguage>,
    deploy_toolkit: &Option<String>,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<Self> {
    let action = match &self.action {
      Action::Custom(cmd) => Action::Custom(cmd.prompt_setup_for_project(&self.info, variables, artifacts)?),
      Action::Check(cmd) => Action::Check(cmd.prompt_setup_for_project(&self.info, variables, artifacts)?),
      Action::PreBuild(pb_action) => {
        Action::PreBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?)
      }
      Action::Build(b_action) => Action::Build(self.setup_buildlike_action(b_action, langs, variables, artifacts)?),
      Action::PostBuild(pb_action) => {
        Action::PostBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?)
      }
      Action::Test(t_action) => Action::Test(self.setup_buildlike_action(t_action, langs, variables, artifacts)?),
      Action::Pack(p_action) => Action::Pack(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::Deliver(p_action) => {
        Action::Deliver(self.setup_packlike_action(p_action, targets, variables, artifacts)?)
      }
      Action::Install(p_action) => {
        Action::Install(self.setup_packlike_action(p_action, targets, variables, artifacts)?)
      }
      Action::ConfigureDeploy(cd_action) => {
        Action::ConfigureDeploy(self.setup_deploylike_action(cd_action, deploy_toolkit, variables, artifacts)?)
      }
      Action::Deploy(d_action) => {
        Action::Deploy(self.setup_deploylike_action(d_action, deploy_toolkit, variables, artifacts)?)
      }
      Action::PostDeploy(pd_action) => {
        Action::PostDeploy(self.setup_deploylike_action(pd_action, deploy_toolkit, variables, artifacts)?)
      }
      Action::Observe(o_action) => Action::Observe(self.setup_observe_action(o_action, variables, artifacts)?),
      Action::Interrupt
      | Action::Patch(_)
      | Action::UseFromStorage(_)
      | Action::AddToStorage(_)
      | Action::SyncToRemote(_)
      | Action::SyncFromRemote(_) => self.action.clone(),
    };

    let mut described_action = self.clone();
    described_action.action = action;

    Ok(described_action)
  }
}

impl CheckAction {
  pub fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<Self> {
    let mut r = self.clone();
    r.command = r.command.prompt_setup_for_project(info, variables, artifacts)?;
    Ok(r)
  }
}

impl CustomCommand {
  pub fn prompt_setup_for_project(
    &self,
    info: &ActionInfo,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<Self> {
    use inquire::{Confirm, Select, Text};

    const USE_ANOTHER: &str = i18n::VAR_SPECIFY_ANOTHER;

    if self.placeholders.as_ref().is_none_or(|ps| ps.is_empty()) {
      return Ok(self.clone());
    }

    println!("{}", i18n::CMD_SPECIFY_VARS.replace("{}", &info.to_str().blue()));

    let mut all_variables = variables.titles();
    let afs = artifacts
      .iter()
      .map(|v| v.to_str().unwrap().to_owned())
      .collect::<Vec<_>>();
    all_variables.extend_from_slice(&afs);
    all_variables.push(USE_ANOTHER.to_string());

    let mut replacements = vec![];
    let mut explicitly_show_bash_c = None;
    loop {
      let mut replacement = vec![];
      for placeholder in self.placeholders.as_ref().unwrap() {
        let mut selected = Select::new(
          &i18n::CMD_SELECT_TO_REPLACE
            .replace("{1}", &placeholder.green())
            .replace("{2}", &self.bash_c.green()),
          all_variables.clone(),
        )
        .prompt()?;

        if variables.is_secret(selected.as_str()) {
          println!("{}", i18n::CMD_HIDDEN_VAR);
          explicitly_show_bash_c = Some(false);
        }

        if selected.as_str() == USE_ANOTHER {
          selected = Text::new(
            &i18n::CMD_SELECT_TO_REPLACE
              .replace("{1}", &placeholder.green())
              .replace("{2}", &self.bash_c.green()),
          )
          .prompt()?;
        }

        replacement.push((
          placeholder.to_owned(),
          variables
            .find(&selected)
            .unwrap_or_else(|| Variable::new_plain(&selected, &selected)),
        ));
      }

      replacements.push(replacement);
      if !Confirm::new(i18n::CMD_ONE_MORE_TIME).with_default(false).prompt()? {
        break;
      }
    }

    let mut r = self.clone();
    r.replacements = Some(replacements);
    r.show_bash_c = if let Some(show) = explicitly_show_bash_c {
      show
    } else {
      r.show_bash_c
    };
    Ok(r)
  }
}

pub fn specify_pipeline_short_name(config: &mut DeployerProjectOptions, short_name: &mut String) -> anyhow::Result<()> {
  while config.pipelines.iter().any(|p| p.title.as_str() == short_name)
    && !inquire::Confirm::new(&i18n::PIPELINE_SHORT_NAME_FOR_PROJECT_OVERRIDE.replace("{}", short_name.as_str()))
      .prompt()?
  {
    *short_name = inquire::Text::new(&format!("{} {}:", i18n::PIPELINE_SHORT_NAME_FOR_PROJECT, i18n::HIT_ESC))
      .prompt_skippable()?
      .ok_or_else(|| anyhow::anyhow!("Hitted Escape."))?;
  }

  Ok(())
}
