use crate::model::AppError;
use std::collections::HashMap;

/// Abstraction over running an external command, so source logic is testable.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, AppError>;
}

/// Runs real processes via std::process::Command.
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, AppError> {
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|e| AppError::Backend(format!("spawn {program}: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Backend(format!(
                "{program} exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Test double: returns canned stdout keyed by the program name.
pub struct FakeRunner {
    pub responses: HashMap<String, Result<String, AppError>>,
}

impl FakeRunner {
    pub fn new() -> Self {
        Self { responses: HashMap::new() }
    }
    pub fn with(mut self, program: &str, out: &str) -> Self {
        self.responses.insert(program.to_string(), Ok(out.to_string()));
        self
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, _args: &[&str]) -> Result<String, AppError> {
        self.responses
            .get(program)
            .cloned()
            .unwrap_or_else(|| Err(AppError::Backend(format!("no fake for {program}"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_runner_returns_canned_output() {
        let r = FakeRunner::new().with("dpkg-query", "hello\n");
        assert_eq!(r.run("dpkg-query", &["-W"]).unwrap(), "hello\n");
        assert!(r.run("missing", &[]).is_err());
    }
}
