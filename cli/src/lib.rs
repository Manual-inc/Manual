use std::fmt;
use std::path::Path;

use manual_core::workspace_descriptor;

#[derive(Debug)]
pub enum CliError {
    MissingCommand,
    MissingPath,
    UnknownCommand(String),
    MissingSkillFile(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "no command provided"),
            Self::MissingPath => write!(f, "missing path argument"),
            Self::UnknownCommand(command) => write!(f, "unknown command: {command}"),
            Self::MissingSkillFile(path) => write!(f, "missing SKILL.md under {path}"),
        }
    }
}

impl std::error::Error for CliError {}

pub fn run<I, S>(args: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let command = args.next().ok_or(CliError::MissingCommand)?;

    match command.as_ref() {
        "about" => Ok(about()),
        "validate-skill" => {
            let path = args.next().ok_or(CliError::MissingPath)?;
            validate_skill(Path::new(path.as_ref()))
        }
        other => Err(CliError::UnknownCommand(other.to_string())),
    }
}

pub fn about() -> String {
    let descriptor = workspace_descriptor();

    format!(
        "{} workspace packages: {}",
        descriptor.name,
        descriptor.packages.join(", ")
    )
}

pub fn validate_skill(path: &Path) -> Result<String, CliError> {
    let skill_file = path.join("SKILL.md");

    if !skill_file.is_file() {
        return Err(CliError::MissingSkillFile(path.display().to_string()));
    }

    Ok(format!("validated skill template at {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn about_command_reports_workspace() {
        let output = run(["cli", "about"]).expect("about command should succeed");

        assert!(output.contains("manual workspace packages"));
    }
}
