use crate::{
    model::{Detection, Result, SshKey},
    process, storage,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn home() -> Result<PathBuf> {
    dirs::home_dir().ok_or("Cannot locate home directory".into())
}
pub fn expand(value: &str) -> Result<PathBuf> {
    let path = if let Some(tail) = value.strip_prefix("~/") {
        home()?.join(tail)
    } else {
        PathBuf::from(value)
    };
    if !path.is_absolute() {
        return Err("Choose an absolute path".into());
    }
    Ok(path)
}
pub fn inspect_key(public: &Path, agent: &str) -> Result<SshKey> {
    if public.extension().and_then(|s| s.to_str()) != Some("pub") {
        return Err("Select an OpenSSH .pub file; private keys are never read".into());
    }
    let bytes = storage::read_optional(public)?.ok_or("Public key does not exist")?;
    if bytes.len() > 16_384 {
        return Err("Public key is too large".into());
    }
    let content = String::from_utf8(bytes).map_err(|_| "Invalid public key")?;
    let kind = content.split_whitespace().next().unwrap_or("");
    if ![
        "ssh-ed25519",
        "ssh-rsa",
        "ecdsa-sha2-nistp256",
        "ecdsa-sha2-nistp384",
        "ecdsa-sha2-nistp521",
        "sk-ssh-ed25519@openssh.com",
        "sk-ecdsa-sha2-nistp256@openssh.com",
    ]
    .contains(&kind)
        || content.trim().lines().count() != 1
    {
        return Err("Unsupported OpenSSH public key".into());
    }
    let output = process::run(
        "ssh-keygen",
        &[
            "-l",
            "-E",
            "sha256",
            "-f",
            public.to_str().ok_or("Invalid key path")?,
        ],
        None,
    )?;
    if output.code != 0 {
        return Err("OpenSSH rejected the public key".into());
    }
    let fingerprint = output
        .stdout
        .split_whitespace()
        .nth(1)
        .ok_or("Missing key fingerprint")?
        .to_string();
    let private = public.with_extension("");
    let metadata = fs::symlink_metadata(&private).map_err(|_| "Matching private key is missing")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Private key must be a regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(
                "Private key permissions are too open; use chmod 600 before importing".into(),
            );
        }
    }
    Ok(SshKey {
        private_path: private.to_string_lossy().into(),
        public_path: public.to_string_lossy().into(),
        public_key: content.trim().into(),
        key_type: kind.into(),
        in_agent: agent.contains(&fingerprint),
        fingerprint,
    })
}

fn valid_key_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && !name.starts_with(['.', '-'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Creates a new Ed25519 key pair in `~/.ssh`, refusing to overwrite existing files.
pub fn generate_key(name: &str, comment: &str) -> Result<SshKey> {
    if !valid_key_name(name) || name.ends_with(".pub") {
        return Err(
            "Use 1–64 letters, digits, dots, hyphens or underscores for the key name.".into(),
        );
    }
    if comment.len() > 200 || comment.chars().any(char::is_control) {
        return Err(
            "Key comment cannot exceed 200 characters or contain control characters.".into(),
        );
    }
    let dir = home()?.join(".ssh");
    if !dir.exists() {
        storage::private_dir(&dir)?;
    }
    let private = dir.join(name);
    let public = dir.join(format!("{name}.pub"));
    if fs::symlink_metadata(&private).is_ok() || fs::symlink_metadata(&public).is_ok() {
        return Err(format!(
            "A key named {name} already exists. Choose another name."
        ));
    }
    let output = process::run(
        "ssh-keygen",
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            comment,
            "-f",
            private.to_str().ok_or("Invalid key path")?,
        ],
        None,
    )?;
    if output.code != 0 {
        return Err(
            "OpenSSH could not generate the key. Check that ssh-keygen is installed.".into(),
        );
    }
    inspect_key(&public, "")
}

pub fn detect() -> Result<Detection> {
    let ssh_dir = home()?.join(".ssh");
    let config = ssh_dir.join("config");
    let version = |program: &str, args: &[&str]| {
        process::run(program, args, None)
            .ok()
            .filter(|o| o.code == 0)
            .map(|o| {
                if o.stdout.is_empty() {
                    o.stderr
                } else {
                    o.stdout
                }
            })
    };
    let agent = process::run("ssh-add", &["-l", "-E", "sha256"], None).ok();
    let agent_text = agent.as_ref().map(|a| a.stdout.as_str()).unwrap_or("");
    let mut keys = vec![];
    let mut warnings = vec![];
    if let Ok(entries) = fs::read_dir(&ssh_dir) {
        for entry in entries.flatten().take(256) {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("pub") {
                match inspect_key(&entry.path(), agent_text) {
                    Ok(key) => keys.push(key),
                    Err(e) => {
                        warnings.push(format!("{}: {e}", entry.file_name().to_string_lossy()))
                    }
                }
            }
        }
    }
    let ssh_text = match storage::read_optional(&config) {
        Ok(Some(bytes)) => String::from_utf8(bytes).unwrap_or_default(),
        Ok(None) => String::new(),
        Err(e) => {
            warnings.push(e);
            String::new()
        }
    };
    let hosts = ssh_text
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()?.eq_ignore_ascii_case("host") {
                Some(
                    words
                        .filter(|w| !w.contains(['*', '?', '!']))
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect();
    let gh_available = version("gh", &["--version"]).is_some();
    let mut github_users = vec![];
    if gh_available {
        if let Ok(output) = process::run("gh", &["auth", "status", "--json", "hosts"], None) {
            // Return only usernames. Never forward raw gh output, tokens or errors.
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                if let Some(users) = value
                    .pointer("/hosts/github.com")
                    .and_then(|v| v.as_array())
                {
                    for user in users {
                        if let Some(login) = user.get("login").and_then(|v| v.as_str()) {
                            github_users.push(login.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(Detection {
        git_version: version("git", &["--version"]),
        ssh_version: version("ssh", &["-V"]),
        git_name: process::git(&["config", "--global", "--get", "user.name"], None)
            .unwrap_or_default(),
        git_email: process::git(&["config", "--global", "--get", "user.email"], None)
            .unwrap_or_default(),
        ssh_config_path: config.to_string_lossy().into(),
        ssh_config_exists: config.exists(),
        ssh_hosts: hosts,
        ssh_keys: keys,
        agent_available: agent.map(|a| a.code == 0 || a.code == 1).unwrap_or(false),
        gh_available,
        github_users,
        platform: std::env::consts::OS.into(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_names_stay_inside_ssh_directory() {
        assert!(valid_key_name("id_ed25519_work"));
        assert!(valid_key_name("github-work.2026"));
        for name in ["", "../id", "a/b", r"a\b", ".hidden", "-flag", "has space"] {
            assert!(!valid_key_name(name), "{name}");
        }
    }
}
