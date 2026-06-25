use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub op_id: String,
    pub stream: String,
    pub line: String,
}

#[cfg(windows)]
fn hide_window(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut std::process::Command) {}

fn build(program: &str, args: &[String]) -> Command {
    let mut std_cmd = std::process::Command::new(program);
    std_cmd.args(args);
    hide_window(&mut std_cmd);
    Command::from(std_cmd)
}

pub async fn capture(program: &str, args: &[String]) -> anyhow::Result<String> {
    let output = build(program, args).output().await.map_err(|e| {
        anyhow::anyhow!("failed to run `{program}`: {e}")
    })?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn capture_ok(program: &str, args: &[String]) -> anyhow::Result<String> {
    let output = build(program, args).output().await.map_err(|e| {
        anyhow::anyhow!("failed to run `{program}`: {e}")
    })?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`{program}` exited with {}: {}", output.status, err.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn stream(
    app: &AppHandle,
    op_id: &str,
    event: &str,
    program: &str,
    args: &[String],
) -> anyhow::Result<i32> {
    let mut cmd = build(program, args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to start `{program}`: {e}"))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let mut out_lines = BufReader::new(stdout).lines();
    let mut err_lines = BufReader::new(stderr).lines();

    let err_task = {
        let app = app.clone();
        let op_id = op_id.to_string();
        let event = event.to_string();
        tokio::spawn(async move {
            while let Ok(Some(line)) = err_lines.next_line().await {
                let _ = app.emit(
                    &event,
                    LogLine { op_id: op_id.clone(), stream: "stderr".into(), line },
                );
            }
        })
    };

    while let Ok(Some(line)) = out_lines.next_line().await {
        let _ = app.emit(
            event,
            LogLine { op_id: op_id.to_string(), stream: "stdout".into(), line },
        );
    }

    let _ = err_task.await;
    let status = child.wait().await?;
    Ok(status.code().unwrap_or(-1))
}

pub fn ps_args(script: &str) -> Vec<String> {
    let wrapped = format!(
        "$ProgressPreference='SilentlyContinue'; \
         [Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}"
    );
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-ExecutionPolicy".into(),
        "Bypass".into(),
        "-Command".into(),
        wrapped,
    ]
}
