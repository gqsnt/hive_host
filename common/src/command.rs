use std::process::Stdio;
use tokio::process::Command as TokioCommand;

pub async fn run_external_command(
    program: &str,
    args: &[&str],
) -> Result<String, tokio::io::Error> {
    let output = TokioCommand::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(tokio::io::Error::other(format!(
            "Command failed: {}. Stderr: {}",
            program,
            stderr.trim()
        )));
    }

    Ok(stdout)
}
