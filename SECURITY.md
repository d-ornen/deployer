# Security Policy

## Deployer Security Considerations

You have to be aware that executing commands on your machine can lead to any consequences! To see all shell commands of the project Pipelines, just exec `deployer cat project -n`.

During Pipeline execution, Deployer prints prepared shell commands on the screen. By the way, these commands may contain secret variables. To hide commands, specify `show_bash_c` field of custom command as `false`.

Do not save secrets as plain project variables! Consider to use environment variables or HashiCorp Vault KV2-storage.

By now, Deployer supports only key-based SSH authorization.

Deployer performs builds in local cache directory. It needs no superuser rights until your command is needed them. But, unfortunately, by now there is no way to simply specify user or superuser to execute some commands.

## Reporting a Vulnerability

If you find a bug that could compromise user security, please report it at Issues and I'll try to fix it it within one to two weeks. If you fixed the bug yourself, please submit a pull request and it will be reviewed in three days.

If you see a big security problem and want to suggest a mechanism for secure interaction with extensions or other external entities, write about it at Issues. Your application can only be rejected if your idea is incredibly difficult to implement, or it does not provide other ways to implement some of the existing features.
