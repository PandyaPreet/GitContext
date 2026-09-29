use crate::model::Result;
use std::{
    fs::File,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

// Output goes to anonymous temporary files, never to logs. This avoids pipe deadlock.
pub fn run(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output> {
    let stdout = tempfile::tempfile().map_err(|_| "Cannot allocate process output")?;
    let stderr = tempfile::tempfile().map_err(|_| "Cannot allocate process output")?;
    let mut command = Command::new(program);
    // Repository inspection must not execute a repository-provided fsmonitor hook.
    if program == "git" {
        command.args(["-c", "core.fsmonitor=false"]);
    }
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(|_| "Cannot clone output")?)
        .stderr(stderr.try_clone().map_err(|_| "Cannot clone output")?)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("LC_ALL", "C");
    // Prevent an inherited terminal context from redirecting config operations.
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
        "GIT_SSH_COMMAND",
        "GIT_SSH",
    ] {
        command.env_remove(name);
    }
    #[cfg(test)]
    {
        command
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }
    if let Some(path) = cwd {
        command.current_dir(path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|_| format!("{program} is unavailable; check installation and PATH"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| "Cannot inspect process")? {
            break status;
        }
        let too_large = [&stdout, &stderr]
            .iter()
            .any(|f| f.metadata().map(|m| m.len() > 1_048_576).unwrap_or(true));
        if start.elapsed() > Duration::from_secs(18) || too_large {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program} exceeded its time or output limit"));
        }
        thread::sleep(Duration::from_millis(30));
    };
    fn read(mut file: File) -> Result<String> {
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))
            .map_err(|_| "Cannot seek output")?;
        let mut value = String::new();
        file.take(1_048_576)
            .read_to_string(&mut value)
            .map_err(|_| "Invalid command output")?;
        Ok(value.trim().to_string())
    }
    Ok(Output {
        code: status.code().unwrap_or(-1),
        stdout: read(stdout)?,
        stderr: read(stderr)?,
    })
}

pub fn git(args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let out = run("git", args, cwd)?;
    if out.code != 0 {
        return Err("Git operation failed. Check repository access and configuration.".into());
    }
    Ok(out.stdout)
}
