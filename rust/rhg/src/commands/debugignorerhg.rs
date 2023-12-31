use crate::error::CommandError;
use hg;
use hg::matchers::get_ignore_matcher;
use hg::StatusError;
use log::warn;

pub const HELP_TEXT: &str = "
Show effective hgignore patterns used by rhg.

This is a pure Rust version of `hg debugignore`.

Some options might be missing, check the list below.
";

pub fn args() -> clap::Command {
    clap::command!("debugignorerhg").about(HELP_TEXT)
}

pub fn run(invocation: &crate::CliInvocation) -> Result<(), CommandError> {
    let repo = invocation.repo?;

    let ignore_file = repo.working_directory_vfs().join(".hgignore"); // TODO hardcoded

    let (ignore_matcher, warnings) = get_ignore_matcher(
        vec![ignore_file],
        repo.working_directory_path(),
        &mut |_source, _pattern_bytes| (),
    )
    .map_err(StatusError::from)?;

    if !warnings.is_empty() {
        warn!("Pattern warnings: {:?}", &warnings);
    }

    let patterns = ignore_matcher.debug_get_patterns();
    invocation.ui.write_stdout(patterns)?;
    invocation.ui.write_stdout(b"\n")?;
    Ok(())
}
