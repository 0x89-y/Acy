use serde::Serialize;
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::process::Command;

const READ_TIMEOUT_SECS: u64 = 300;

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
    let mut cmd = Command::from(std_cmd);
    cmd.kill_on_drop(true);
    cmd
}

async fn output_with_timeout(
    program: &str,
    mut cmd: Command,
) -> anyhow::Result<std::process::Output> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to run `{program}`: {e}"))?;
    match tokio::time::timeout(Duration::from_secs(READ_TIMEOUT_SECS), child.wait_with_output())
        .await
    {
        Ok(res) => res.map_err(|e| anyhow::anyhow!("failed to run `{program}`: {e}")),
        Err(_) => anyhow::bail!(
            "`{program}` did not respond within {READ_TIMEOUT_SECS}s - it may be waiting on a \
             source agreement or prompt. Try running `{program}` once in a terminal (e.g. \
             `winget list`), or install the WinGet PowerShell module in Settings \u{2192} Sources."
        ),
    }
}

pub async fn capture(program: &str, args: &[String]) -> anyhow::Result<String> {
    let output = output_with_timeout(program, build(program, args)).await?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn capture_ok(program: &str, args: &[String]) -> anyhow::Result<String> {
    let output = output_with_timeout(program, build(program, args)).await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`{program}` exited with {}: {}", output.status, err.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// Read one line as lossy UTF-8. `lines()` errors on invalid UTF-8 (choco writes
// in the OEM code page), which used to stop the read loop while the pipe stayed
// open - the child then blocked on a full pipe and the op hung forever.
async fn next_line<R: AsyncBufRead + Unpin>(reader: &mut R, buf: &mut Vec<u8>) -> Option<String> {
    buf.clear();
    match reader.read_until(b'\n', buf).await {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(
            String::from_utf8_lossy(buf)
                .trim_end_matches(['\r', '\n'])
                .to_string(),
        ),
    }
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

    let mut out_reader = BufReader::new(stdout);
    let mut err_reader = BufReader::new(stderr);

    let err_task = {
        let app = app.clone();
        let op_id = op_id.to_string();
        let event = event.to_string();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            while let Some(line) = next_line(&mut err_reader, &mut buf).await {
                let _ = app.emit(
                    &event,
                    LogLine { op_id: op_id.clone(), stream: "stderr".into(), line },
                );
            }
        })
    };

    let mut buf = Vec::new();
    while let Some(line) = next_line(&mut out_reader, &mut buf).await {
        let _ = app.emit(
            event,
            LogLine { op_id: op_id.to_string(), stream: "stdout".into(), line },
        );
    }

    let _ = err_task.await;
    let status = child.wait().await?;
    Ok(status.code().unwrap_or(-1))
}

/// Quote a value as a PowerShell single-quoted string literal. PowerShell treats
/// the curly quotes U+2018..U+201B as single quotes too, so they're doubled
/// alongside `'` - otherwise `x’; calc; ’` would break out of the string.
pub fn ps_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
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
        "RemoteSigned".into(),
        "-Command".into(),
        wrapped,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_quote_doubles_every_single_quote_kind() {
        assert_eq!(ps_quote("vlc"), "'vlc'");
        assert_eq!(ps_quote("it's"), "'it''s'");
        assert_eq!(ps_quote("x\u{2019}; calc; \u{2019}"), "'x\u{2019}\u{2019}; calc; \u{2019}\u{2019}'");
        assert_eq!(ps_quote("\u{2018}\u{201A}\u{201B}"), "'\u{2018}\u{2018}\u{201A}\u{201A}\u{201B}\u{201B}'");
    }

    #[tokio::test]
    async fn next_line_survives_invalid_utf8() {
        let data: &[u8] = b"ok\r\nJ\xfcrgen\nlast";
        let mut reader = BufReader::new(data);
        let mut buf = Vec::new();
        assert_eq!(next_line(&mut reader, &mut buf).await.as_deref(), Some("ok"));
        assert_eq!(next_line(&mut reader, &mut buf).await.as_deref(), Some("J\u{fffd}rgen"));
        assert_eq!(next_line(&mut reader, &mut buf).await.as_deref(), Some("last"));
        assert_eq!(next_line(&mut reader, &mut buf).await, None);
    }
}
