use crate::types::TargetCommand;
use anyhow::{bail, Context};

pub(crate) fn parse_json_argv_flag(
    flag: &'static str,
    raw: Option<String>,
) -> anyhow::Result<Option<Vec<String>>> {
    raw.map(|value| {
        let argv = serde_json::from_str::<Vec<String>>(&value)
            .with_context(|| format!("{flag} must be valid JSON string array"))?;
        validate_argv(flag, &argv)?;
        Ok(argv)
    })
    .transpose()
}

pub(crate) fn parse_required_command_form(
    labels: CommandFormLabels,
    shell: Option<String>,
    argv: Option<Vec<String>>,
) -> anyhow::Result<TargetCommand> {
    parse_command_form(labels, shell, argv, true)?.with_context(|| {
        format!(
            "{} command form is required; provide exactly one of {} or {}",
            labels.role, labels.shell, labels.argv
        )
    })
}

pub(crate) fn parse_optional_command_form(
    labels: CommandFormLabels,
    shell: Option<String>,
    argv: Option<Vec<String>>,
) -> anyhow::Result<Option<TargetCommand>> {
    parse_command_form(labels, shell, argv, false)
}

fn parse_command_form(
    labels: CommandFormLabels,
    shell: Option<String>,
    argv: Option<Vec<String>>,
    required: bool,
) -> anyhow::Result<Option<TargetCommand>> {
    match (shell, argv) {
        (Some(_), Some(_)) if required => bail!(
            "provide exactly one of {} or {} for {}",
            labels.shell,
            labels.argv,
            labels.role
        ),
        (Some(_), Some(_)) => bail!(
            "provide at most one of {} or {} for {}",
            labels.shell,
            labels.argv,
            labels.role
        ),
        (Some(command), None) => {
            if labels.reject_empty_shell && command.trim().is_empty() {
                bail!("{} must not be empty", labels.shell);
            }
            Ok(Some(TargetCommand::Shell(command)))
        }
        (None, Some(argv)) => {
            validate_argv(labels.argv, &argv)?;
            Ok(Some(TargetCommand::Argv(argv)))
        }
        (None, None) => Ok(None),
    }
}

pub(crate) fn validate_argv(label: &str, argv: &[String]) -> anyhow::Result<()> {
    if argv.is_empty() {
        bail!("{label} must not be empty");
    }
    if argv[0].trim().is_empty() {
        bail!("{label} command must not be empty");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandFormLabels {
    pub(crate) role: &'static str,
    pub(crate) shell: &'static str,
    pub(crate) argv: &'static str,
    pub(crate) reject_empty_shell: bool,
}
