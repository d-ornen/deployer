//! English internalization module.
//!
//! Enabled by default, disabled when `i18n-ru` feature is enabled.

use crate::tr;

// Check
tr!(CHECK_IGNORE_FAILS, "Does the command failure also means check failure?");

tr!(CHECK_CURR_REGEX, "Current regexes are:");
tr!(SPECIFY_REGEX_SUCC, "Specify success when found some regex?");
tr!(SPECIFY_REGEX_FAIL, "Specify success when NOT found some regex?");
tr!(SPECIFY_REGEX_FOR_SUCC, "for success on match");
tr!(SPECIFY_REGEX_FOR_FAIL, "for success on mismatch");
tr!(
  CHECK_NEED_TO_AT_LEAST,
  "For `Check` Action you need to specify at least one regex check!"
);
tr!(CHECK_SPECIFY_WHAT, "Specify an action for Check Action:");
tr!(CHECK_EDIT_CMD, "Edit check command");
tr!(CHECK_EDIT_REGEXES, "Edit regexes");
tr!(CHECK_ENTER_REGEX, "Enter regex");
tr!(CHECK_HELP, "(or enter '/h' for help)");
tr!(CHECK_REGEX_INVALID_DUE, "The regex you've written is invalid due to");

tr!(GUIDE, "Guide");

tr!(CHECK_GUIDE_TITLE, "Regex Checks for Deployer");
tr!(CHECK_GUIDE_1, "The usage of regex checks in Deployer is simple enough.");
tr!(
  CHECK_GUIDE_2,
  "If you want to specify some text that needed to be found, you simply write this text."
);
tr!(CHECK_GUIDE_3, "For finding an info about any supported regex read this");
tr!(CHECK_GUIDE_4, "For checks use this");
tr!(CHECK_GUIDE_5, "select `Rust` flavor at left side panel");

tr!(PATTERN, "Pattern");
tr!(FOUND, "found");
tr!(NOT_FOUND, "not found");

// Custom Command
tr!(CMD_SPECIFY_BASH_C, "Enter a command for terminal");
tr!(CMD_PLACEHOLDERS, "Enter command placeholders, if any:");
tr!(CMD_IGNORE_FAILS, "Ignore command failures?");
tr!(CMD_SHOW_BASH_C, "Show an entire command at Pipeline's run?");
tr!(
  CMD_SHOW_SUCC_OUT,
  "Show an output of command if it executed successfully?"
);
tr!(CMD_ONLY_WHEN_FRESH, "Start a command only in fresh runs?");
tr!(CMD_DAEMON, "Run as a daemon until the Pipeline is complete?");

tr!(CUSTOM_CMD_GUIDE_TITLE, "Shell Commands for Deployer");
tr!(
  CUSTOM_CMD_GUIDE_1,
  "The usage of shell commands in Deployer is very simple."
);
tr!(
  CUSTOM_CMD_GUIDE_2,
  "You can use `%1%` for home directories, your default `%2%` variable and so on."
);
tr!(
  CUSTOM_CMD_GUIDE_3,
  "Also you can write your commands even when there are some unspecified variables:"
);
tr!(
  CUSTOM_CMD_GUIDE_4,
  "To specify shell for Deployer, use `DEPLOYER_SH_PATH` environment variable."
);
tr!(CUSTOM_CMD_GUIDE_5, "By default: using");
tr!(CUSTOM_CMD_GUIDE_6, "Now: using");

tr!(CUSTOM_CMD_EDIT, "Edit command");
tr!(CUSTOM_CMD_REORDER, "Reorder commands");
tr!(CUSTOM_CMD_ADD, "Add command");
tr!(CUSTOM_CMD_RM, "Remove command");

tr!(CMD_SPECIFY_VARS, "Specifying variables for `{}` Action:");
tr!(
  CMD_SELECT_TO_REPLACE,
  "Select variable to replace `{1}` in `{2}` shell command:"
);
tr!(
  CMD_HIDDEN_VAR,
  "At run stage the command will be hidden due to usage of secret variable."
);
tr!(
  CMD_ONE_MORE_TIME,
  "Enter `y` if you need exec this command one more time with others variables."
);
tr!(CMD_SELECT_TO_CHANGE, "Select an option to change in `{}` command");
tr!(CMD_SELECT_TO_REMOVE, "Select a command to remove:");

tr!(CMD_EDIT_SHELL, "Edit shell command");
tr!(CMD_CHANGE_PLACEHOLDERS, "Change command placeholders");
tr!(CMD_CHANGE_FAILURE_IGNORANCE, "Change command failure ignorance");
tr!(
  CMD_CHANGE_VISIBILITY_AT_BUILD,
  "Change whether command is displayed or not on run stage"
);
tr!(
  CMD_CHANGE_VISIBILITY_ON_SUCC,
  "Change whether command output is displayed or not when it executed successfully"
);
tr!(CMD_CHANGE_ON_FRESH, "Change command executing only at fresh runs");

tr!(CMDS_REORDER, "Reorder Action's commands:");

tr!(CMD_SKIP_DUE_TO_NOT_FRESH, "Skip a command due to not a fresh run...");
tr!(EXECUTING, "Executing");
tr!(EXECUTING_HIDDEN, "Executing the command:");
tr!(ERRORS, "Errors:");

tr!(HIT_ESC, "(hit `esc` when done)");
tr!(OR_HIT_ESC, "(or hit `esc`)");

tr!(
  CUSTOM_CMD_EDIT_PROMPT,
  "Select a concrete command to change (hit `esc` when done):"
);

// PL
tr!(PL_INPUT_PROMPT, "Input the programming language name:");
tr!(LANGUAGE, "Language");

tr!(PL_ACTION_PROMPT, "Select a concrete language to change");
tr!(ADD, "Add");
tr!(REMOVE, "Remove");
tr!(REORDER, "Reorder");
tr!(PL_TO_REMOVE, "Select a language to remove:");
tr!(PL_SELECT, "Select programming languages:");
tr!(
  PL_COLLECT,
  "Enter the names of programming languages separated by commas:"
);

// Targets
tr!(TARGET_ARCH, "Enter the target's architecture:");
tr!(TARGET_OS_SELECT, "Select OS:");
tr!(TARGET_OS_OTHER, "Enter OS name:");
tr!(TARGET_OS_DER, "Enter OS derivative:");
tr!(TARGET_OS_VER_S, "Select version specification type:");

tr!(TARGET_OS_VER_NS, "Not Specified");
tr!(TARGET_OS_VER_WS, "Weak Specified");
tr!(TARGET_OS_VER_SS, "Strong Specified");

tr!(TARGET_OS_VER, "Enter version:");

tr!(EDIT_ACTION_PROMPT, "Select an edit action");
tr!(EDIT_ARCH, "Edit arch");
tr!(EDIT_OS, "Edit OS");
tr!(EDIT_TITLE, "Edit title");
tr!(EDIT_VAR_SECRET, "Change secret flag");
tr!(EDIT_VALUE, "Edit value");

// Variables
tr!(VAR, "Variable");
tr!(VAR_TITLE, "Enter your variable's title:");
tr!(NOTE, "Note");
tr!(
  VAR_NOTE,
  "if variable is a secret, then no command containing this variable will be printed during the run stage."
);
tr!(
  KV2_NOTE,
  "before run, you must specify two environment variables for the Deployer:"
);
tr!(VAR_IS_SECRET, "Is this variable a secret?");
tr!(SPECIFY_VAR_TYPE, "Select the variable type:");
tr!(VAR_PLAIN, "Simple variable");
tr!(VAR_ENVF, "Variable from ENV file");
tr!(VAR_ENV, "Variable from environment");
tr!(VAR_KV2, "Variable from HashiCorp Vault KV2 storage");
tr!(VAR_PLAIN_CONTENT, "Enter the variable's content:");
tr!(VAR_ENV_FILE, "Enter the ENV file path:");
tr!(VAR_ENV_KEY, "Enter the variable's key:");
tr!(VAR_MOUNT_PATH, "Enter the KV2 mount path:");
tr!(VAR_SECRET_PATH, "Enter the secret's path:");

tr!(VAR_EDIT, "Edit variable");
tr!(VAR_SELECT_FC, "Select a concrete variable to change");
tr!(VAR_TO_REMOVE, "Select a variable to remove:");

tr!(VAR_SPECIFY_ANOTHER, "· Specify another variable");
tr!(ACTION_SPECIFY_ANOTHER, "· Specify another Action");
tr!(PIPELINE_SPECIFY_ANOTHER, "· Specify another Pipeline");

// Actions
tr!(ACTION_SHORT_NAME, "Write the Action's short name:");
tr!(ACTION_VERSION, "Specify the Action's version:");
tr!(ACTION_FULL_NAME, "Write the Action's full name:");
tr!(ACTION_DESC, "Write the Action's description:");
tr!(ACTION_TAGS, "Write Action's tags, if any:");

tr!(ACTION_SELECT_TYPE, "Select Action's type (read the docs for details):");
tr!(
  ACTION_REG_ALREADY_HAVE,
  "Actions Registry already have `{}` Action. Do you want to override it? (y/n)"
);
tr!(
  ACTION_COMPAT_PLS,
  "Action `{1}` may be not fully compatible with your project due to requirements (Action's supported langs: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)"
);
tr!(
  ACTION_COMPAT_TARGETS,
  "Action `{1}` may be not fully compatible with your project due to requirements (Action's target: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)"
);
tr!(
  ACTION_COMPAT_DEPL_TOOLKIT,
  "Action `{1}` may be not fully compatible with your project due to requirements (Action's deploy toolkit: {2}, your project's: {3}). Use this Action anyway? If no, Action will be skipped. (y/n)"
);

tr!(ACTION_SELECT_TO_CHANGE, "Select a concrete Action to change");
tr!(ACTION_EDIT, "Edit Action `{1}` - `{2}`");
tr!(ACTION_REMOVE, "Remove Action `{1}` - `{2}`");
tr!(ACTION, "Action `{1}` - `{2}`");
tr!(ACTION_ADD, "Add Action?");

tr!(ACTION_CHOOSE_TO_ADD, "Choose an Action to add:");
tr!(ACTION_CHOOSE_TO_REMOVE, "Select an Action to remove:");
tr!(
  ACTION_REGISTRY_CHOOSE_TO_REMOVE,
  "Select Action for removing from Actions' Registry:"
);

tr!(ARE_YOU_SURE, "Are you sure? (y/n)");

tr!(ADD_CMD, "Add command?");
tr!(EDIT_COMMAND, "Edit command");
tr!(EDIT_COMMANDS, "Edit commands");
tr!(EDIT_PLS, "Edit programming languages");
tr!(EDIT_TARGETS, "Edit targets");
tr!(EDIT_DEPL_TOOLKIT, "Edit deploy toolkit");
tr!(EDIT_DESC, "Edit description");
tr!(EDIT_TAGS, "Edit tags");
tr!(EDIT_EXCL_TAG, "Edit exclusive execution tag");
tr!(EDIT_PIPELINE_ACTIONS, "Edit Pipeline's Actions");
tr!(EDIT_PROJECT_NAME, "Edit project name");
tr!(EDIT_PROJECT_PIPELINES, "Edit project Pipelines");
tr!(EDIT_PROJECT_REASSIGN, "Reassign project variables to Actions");
tr!(EDIT_CACHE, "Edit cache files");
tr!(EDIT_PROJECT_VARS, "Edit project variables");
tr!(EDIT_ARTIFACTS, "Edit artifacts");
tr!(EDIT_AF_INPLACE, "Edit artifact inplacements");
tr!(EDIT_CONF_FORMAT, "Change configuration file format");
// tr!(EDIT_PREFERRED_CONF_FORMAT, "Change default configuration file format");
tr!(EDIT_DEFAULT, "Select default Pipeline");
tr!(EDIT_PATCH, "Edit patch");
tr!(EDIT_ATS, "Edit automatical artifact-to-storage push rules");
tr!(EDIT_REQS, "Edit Action requirements");
tr!(EDIT_REMOTE_SHORT_NAME, "Edit remote host short name");
tr!(
  EDIT_EXEC_IN_PROJECT_DIR,
  "Change Action's execution path (run or project folder)"
);
tr!(EDIT_PIPELINE, "Edit Pipeline");

tr!(DEPL_TOOLKIT, "Enter deploy toolkit name");

tr!(ACTIONS_AVAILABLE, "Available Actions in Deployer's Registry:");
tr!(NO_ACTIONS, "There is no Actions in Registry.");

tr!(TAGS, "tags");

tr!(
  EXEC_IN_PROJECT_DIR,
  "Do you need execute this Action in project's directory instead of run directory?"
);

// Pipelines
tr!(PIPELINE_SHORT_NAME, "Write the Pipeline's short name:");
tr!(PIPELINE_VERSION, "Specify the Pipeline's version:");
tr!(PIPELINE_FULL_NAME, "Write the Pipeline's full name:");
tr!(PIPELINE_DESC, "Write the Pipeline's description:");
tr!(PIPELINE_TAGS, "Write Pipeline's tags, if any:");

tr!(PIPELINE_SPECIFY_EXCL_TAG, "Specify exclusive pipeline tag");

tr!(PIPELINES_AVAILABLE, "Available Pipelines in Deployer's Registry:");
tr!(NO_PIPELINES, "There is no Pipelines in Registry.");

tr!(
  PIPELINE_REG_ALREADY_HAVE,
  "Pipelines Registry already have `{}` Pipeline. Do you want to override it? (y/n)"
);
tr!(REORDER_PIPELINE_ACTIONS, "Reorder Pipeline's Actions:");
tr!(SELECT_ACTION_TO_ADD_TO, "Select Action for adding to Pipeline:");
tr!(
  PIPELINE_DESCRIBE_ACTION_IN,
  "Describe this Action inside your Pipeline:"
);

tr!(GOT_FROM, "Got from");

tr!(PIPELINE_CHOOSE_TO_ADD, "Choose a Pipeline to add:");
tr!(PIPELINE_CHOOSE_TO_REMOVE, "Select a Pipeline to remove:");
tr!(
  PIPELINE_REGISTRY_CHOOSE_TO_REMOVE,
  "Select Pipeline for removing from Pipeline's Registry:"
);
tr!(PIPELINES_REORDER, "Reorder Pipelines inside your project:");

tr!(
  NO_SUCH_PIPELINE,
  "There is no such Pipeline in Registry. See available Pipelines with `deployer ls pipelines`."
);

tr!(CFG_INVALID, "Config is invalid! Reinit the project.");

tr!(PIPELINE_SELECT_FOR_PROJECT, "Select the Pipeline for this project:");
tr!(
  PIPELINE_SHORT_NAME_FOR_PROJECT,
  "Write the Pipeline's short name (only for this project):"
);
tr!(PIPELINE_NEW_DEFAULT, "Set this Pipeline running by default? (y/n)");
tr!(
  PIPELINE_NEW_DEFAULT_REPLACE,
  "Pipeline `{}` is already set by default. Set this Pipeline running by default instead?"
);
tr!(
  PIPELINE_DEFAULT_SET,
  "Pipeline is successfully set up for this project."
);

tr!(
  PIPELINE_SHORT_NAME_FOR_PROJECT_OVERRIDE,
  "Do you want to overwrite an existing pipeline `{}` for this project? (y/n)"
);
tr!(PIPELINE_EDIT, "Edit Pipeline `{1}` - `{2}`");
tr!(SELECT_PIPELINE_TO_CHANGE, "Select a concrete Pipeline to change");
tr!(PIPELINE, "Pipeline");
tr!(PIPELINE_REORDER_ACTIONS, "Reorder Pipeline's Actions:");
tr!(PIPELINE_REMOVE, "Remove Pipeline `{1}` - `{2}`");
tr!(SELECT_PIPELINE_TO_ADD_TO, "Select a Pipeline:");

tr!(STARTING_PIPELINE, "Starting the `{}` Pipeline...");
tr!(STARTING_ACTION, "Action");
tr!(ARTIFACT_ENPLACE_FAIL, "There is no such artifact");
tr!(INTERRUPT, "The Pipeline is interrupted. Hit `Enter` to continue");

tr!(BUILD_PATH, "Run path");
tr!(DONE_IN, "Done in");

tr!(DONE, " done");
tr!(GOT_ERROR, " got an error!");

// Project
tr!(PROJECT_NAME, "Enter the project's name:");
tr!(
  PROJECT_SPECIFY_PLS,
  "Please, specify the project's programming languages to setup default cache folders."
);
tr!(
  PROJECT_DEPL_TOOLKIT,
  "Specify your deploy toolkit (`docker`, `docker-compose`, `podman`, `k8s`, etc.)"
);

tr!(ENTITY, "Entity");
tr!(VALUE_TO_REMOVE, "Select a value to remove:");
tr!(INPLACEMENT, "Inplacement");

tr!(SELECT_PROJECT_AF, "Select project's artifact:");
tr!(
  CHOOSE_AF_INPLACEMENT,
  "Enter relative path of artifact inplacement (inside `artifacts` subfolder):"
);
tr!(REMOVE_INPLACEMENT, "Select an inplacement to remove:");
tr!(TARGET, "Target");
tr!(EDIT_TARGET, "Edit target");
tr!(SELECT_TARGET_TO_CHANGE, "Select a concrete target to change");
tr!(SELECT_TARGET_TO_REMOVE, "Select a target to remove:");
tr!(ADD_NEW_TARGET, "Add new run target?");

tr!(ADD_NEW_AF, "Add new run artifact?");
tr!(ADD_NEW_VAR, "Add new project-related variable or secret?");
tr!(
  ADD_NEW_INPLACEMENT_FIRST,
  "Do you want to create artifact inplacement from run directory to your project's location (inside `artifacts` subfolder)?"
);
tr!(ADD_NEW_INPLACEMENT_SECOND, "Add one more artifact inplacement?");

tr!(
  INIT_SUCC,
  "Setup is completed. Don't forget to assign at least one Pipeline to the project to run!"
);
tr!(
  PROJECT_NO_PIPELINES,
  "Current project have no specified Pipelines to select the default one."
);
tr!(EDIT_DEFAULT_PROMPT, "Select the default project's Pipeline");
tr!(ENTER_CONF_FORMAT, "Enter the new config format (`yaml`/`json`/`toml`):");
tr!(HIDDEN, "hidden");

// Patch
tr!(PATCH_ERROR, "Patch application failed");
tr!(PATCH_DONE, "The patch has been applied {} time(s).");
tr!(
  PATCH_DONE_ZERO_TIMES,
  "The patch wasn't applied! The contents of the patch are probably incorrect, or the project files have changed."
);
tr!(PATCH_SPECIFY_PATH, "Specify patch location:");

// Content & Storage
tr!(NO_SUCH_CONTENT, "There is no such content");
tr!(
  CONTENT_CONSIDER_ADD,
  "Consider to add this content via `deployer add content`."
);
tr!(CONTENT_INFO, "Write the content's short name:");
tr!(CONTENT_VER, "Specify the content's version:");
tr!(CONTENT_AVAILABLE, "Available content in Deployer's storage:");
tr!(
  CONTENT_GUIDE_1,
  "To add content, you need to specify the path to the content folder."
);
tr!(
  CONTENT_GUIDE_2,
  "The content in it must be located in such a way that the paths to the required files are relative to the run folder."
);
tr!(
  CONTENT_GUIDE_3,
  "For example, if you need to place a Dockerfile at the root of the run folder, you place the file at the root of the content folder; if you need the file to be located in a subfolder, you place it in a subfolder with the same name inside the content folder."
);
tr!(
  CONTENT_GUIDE_4,
  "Now you need to specify the short name and version of the content (for example, `dockerfile` content with version `0.1.0`)."
);
tr!(
  CONTENT_GUIDE_5,
  "You will need this information to add a `UseFromStorage` Action."
);

tr!(CONTENT_SPECIFY_PATH, "Specify content folder's path:");
tr!(
  CONTENT_ADDED_SUCC,
  "Content `{1}` added to Deployer's storage successfully (path: {2})"
);
tr!(CONTENT_SELECT_TO_REMOVE, "Select content version to remove:");
tr!(PATH, "path");

// Paths and resolver
tr!(ABSOLUTE_PATH, "Enter the absolute path:");
tr!(RELATIVE_PATH, "Enter the relative path:");
tr!(
  INCORRECT_PATH,
  "Incorrect path! Entity must be placed inside run folder!"
);
tr!(
  INCORRECT_AF_INPL_PATH,
  "Incorrect artifact's inplacement path! Artifact must be inplaced inside `artifacts` folder!"
);

// Build clean
tr!(CLEANED, "Cleaned");

// Info
tr!(
  INCORRECT_SHORT_NAME,
  "Short names must only contain English characters and `_` and `-` characters."
);
tr!(INCORRECT_VERSION, "Versions must be like this: `1`, `1.2`, or `1.2.3`.");

tr!(
  SPECIFY_SHORT_NAME_FOR_ADD_TO_STORAGE,
  "Specify a short name for the content under which it will be loaded into the storage automatically:"
);
tr!(
  IF_NEEDED_TO_CHANGE_AUTOVER,
  "Do you want to change the way content is automatically versioned? Current method is:"
);
tr!(
  AUTO_VER_CMD_STDOUT,
  "execute the command and get output from `stdout` as version"
);
tr!(AUTO_VER_PLAIN_FILE, "get a version from specified plain file");
tr!(SPECIFY_AUTO_VER, "Select the version detection method:");
tr!(
  SPECIFY_AUTO_VER_RELATIVE_FILEPATH,
  "Specify the relative path to plain version file:"
);

// Requirements
tr!(REQUIREMENT, "Requirement");
tr!(REQ_NOT_SATISFIED, "Requirement `{}` for this Action is not satisfied.");
tr!(
  REQ_CMD_NOT_SATISFIED,
  "Requirement (check) for this Action is not satisfied, output:\n"
);
tr!(
  ADD_REQ,
  "Add any requirement (path check or any Check Action) for this Action?"
);
tr!(REQ_TYPE_EX, "Check some path exists");
tr!(REQ_TYPE_EX_ANY, "Check any of given paths exists");
tr!(REQ_TYPE_CHECK, "Check the output of given command");
tr!(REQ_TYPE_REMOTE, "Check the remote host availability");
tr!(SELECT_REQ_TYPE, "Select the requirement type:");
tr!(ADD_NEW_PATH, "Add new path?");
tr!(REQ_CHECKS_TOOK, "Requirements checked in");

// Remote hosts
tr!(REMOTE_SHORT_NAME, "Specify the remote host's short name:");
tr!(NO_SUCH_REMOTE, "There is no such remote host in Deployer's Registry!");
tr!(HOST_SHORT_NAME, "Remote host short name");
tr!(HOST, "Host");
tr!(HOST_IP, "IP-address");
tr!(HOST_PORT, "Port");
tr!(HOST_USERNAME, "Username");
tr!(KNOWN_HOSTS, "Known remote hosts in Deployer's Registry:");
tr!(SPECIFY_HOST_IP, "Specify the IP-address of a remote host's SSH server:");
tr!(SPECIFY_HOST_PORT, "Specify the port of a remote host's SSH server:");
tr!(SPECIFY_HOST_USERNAME, "Specify the remote username:");
tr!(
  SPECIFY_SSH_KEY_PATH,
  "Specify the path to private SSH key for remote host:"
);
tr!(NO_HOSTS, "There is no remote hosts in Registry.");
tr!(NO_SUCH_HOSTS, "There is no such remote hosts in Registry.");
tr!(
  REMOTE_REGISTRY_CHOOSE_TO_REMOVE,
  "Select remote host for removing from Deployer's Registry:"
);
tr!(REMOTE_EXEC, "Executing at");
tr!(
  REMOTE_ADD_TO_CMD_FIRST,
  "Do you want to execute this command remotely on one or many remote hosts? If yes, the command won't be executed on this host."
);
tr!(REMOTE_ADD_TO_CMD_ANOTHER, "Add one more remote host?");
tr!(START_BUILD_AT_REMOTE, "Starting run on remote host");
tr!(BUILT_AT_REMOTE, "Run at remote and got artifacts from host:");
tr!(REMOTE_NO_DEPLOYER, "There is no Deployer installed remotely.");
tr!(
  REMOTE_CONSIDER_UPGRADE,
  "Deployer versions aren't the same. Consider to update Deployer on your hosts."
);
