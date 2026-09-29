use crate::{
    model::{Profile, Result, Verification},
    process,
};
use std::{io::Write, path::Path};

pub fn validate_profile(profile: &Profile) -> Result<()> {
    for value in [
        &profile.name,
        &profile.git_name,
        &profile.git_email,
        &profile.organization,
    ] {
        if value.len() > 200 || value.chars().any(char::is_control) {
            return Err(
                "Profile fields cannot contain control characters or exceed 200 characters".into(),
            );
        }
    }
    if profile.name.trim().is_empty()
        || profile.git_name.trim().is_empty()
        || !profile.git_email.contains('@')
    {
        return Err("Profile name, Git author and email are required".into());
    }
    crate::provider::validate_endpoint(profile)?;
    let prefix = format!("{}-", profile.provider.label().to_ascii_lowercase());
    if !profile.ssh_alias.starts_with(&prefix)
        || profile.ssh_alias.len() <= prefix.len()
        || profile.ssh_alias.len() > 80
        || !profile
            .ssh_alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(format!(
            "SSH alias must start with {prefix} and contain only letters, digits and hyphens."
        ));
    }
    for path in [&profile.private_key_path, &profile.public_key_path] {
        if path
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '%' | '$' | '`'))
        {
            return Err("Unsupported metacharacter in SSH key path".into());
        }
    }
    if !["mint", "blue", "violet", "amber"].contains(&profile.color.as_str()) {
        return Err("Invalid profile color".into());
    }
    Ok(())
}

pub fn stanza(profile: &Profile) -> String {
    let port = if profile.ssh_port == 22 {
        String::new()
    } else {
        format!("    Port {}\n", profile.ssh_port)
    };
    format!("# Git Context begin {}\nHost {}\n    HostName {}\n    User git\n{}    IdentityFile \"{}\"\n    IdentitiesOnly yes\n    StrictHostKeyChecking yes\nHost *\n# Git Context end {}\n", profile.id, profile.ssh_alias, profile.host, port, profile.private_key_path.replace('\\', "/"), profile.id)
}

pub fn merge_config(existing: &str, profile: &Profile) -> Result<String> {
    let block = stanza(profile);
    let begin = format!("# Git Context begin {}", profile.id);
    let end = format!("# Git Context end {}", profile.id);
    let mut preserved = String::new();
    let mut inside = false;
    let mut found = false;
    let mut managed = String::new();
    for line in existing.split_inclusive('\n') {
        if line.trim_end() == begin {
            if inside || found {
                return Err("Duplicate managed SSH block".into());
            }
            inside = true;
            found = true;
            managed.push_str(line);
            continue;
        }
        if line.trim_end() == end {
            if !inside {
                return Err("Malformed managed SSH block".into());
            }
            inside = false;
            managed.push_str(line);
            continue;
        }
        if inside {
            managed.push_str(line);
            continue;
        }
        let mut words = line
            .trim()
            .split([' ', '\t', '='])
            .filter(|w| !w.is_empty());
        let keyword = words.next().unwrap_or("");
        if keyword.eq_ignore_ascii_case("include") || keyword.eq_ignore_ascii_case("match") {
            return Err("SSH config uses Include or Match. Automatic edits are disabled to preserve its semantics; manage this alias manually.".into());
        }
        if keyword.eq_ignore_ascii_case("host")
            && words.any(|host| host.eq_ignore_ascii_case(&profile.ssh_alias))
        {
            return Err("SSH alias already exists outside this profile's managed block".into());
        }
        preserved.push_str(line);
    }
    if inside {
        return Err("Unclosed managed SSH block".into());
    }
    if found && managed != block {
        return Err("This profile's managed SSH block was edited externally. Review it manually before reapplying.".into());
    }
    // Host * terminates the managed stanza before the original configuration.
    Ok(format!("{block}{preserved}"))
}

pub fn validate_config(text: &str, profile: &Profile, directory: &Path) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(directory).map_err(|_| "Cannot stage SSH configuration")?;
    temp.write_all(text.as_bytes())
        .map_err(|_| "Cannot stage SSH configuration")?;
    let out = process::run(
        "ssh",
        &[
            "-G",
            "-F",
            temp.path().to_str().ok_or("Invalid config path")?,
            &profile.ssh_alias,
        ],
        None,
    )?;
    if out.code != 0 {
        return Err("OpenSSH rejected the proposed configuration".into());
    }
    validate_resolved(&out.stdout, profile)
}

pub fn validate_installed(profile: &Profile, config_path: &Path) -> Result<()> {
    // Default invocation includes administrator-managed system SSH configuration.
    // Explicit paths support isolated fixture/config roots without touching HOME.
    let default_path = crate::detection::home()?.join(".ssh/config");
    let output = if config_path == default_path {
        process::run("ssh", &["-G", &profile.ssh_alias], None)?
    } else {
        process::run(
            "ssh",
            &[
                "-G",
                "-F",
                config_path.to_str().ok_or("Invalid SSH config path")?,
                &profile.ssh_alias,
            ],
            None,
        )?
    };
    if output.code != 0 {
        return Err("OpenSSH could not resolve the installed alias".into());
    }
    validate_resolved(&output.stdout, profile)
}

fn validate_resolved(output: &str, profile: &Profile) -> Result<()> {
    let values = |key: &str| {
        output
            .lines()
            .filter_map(|line| {
                line.split_once(' ')
                    .filter(|(k, _)| *k == key)
                    .map(|(_, value)| value.to_string())
            })
            .collect::<Vec<_>>()
    };
    let expected_path = profile.private_key_path.replace('\\', "/");
    if values("hostname") != [profile.host.as_str()]
        || values("port") != [profile.ssh_port.to_string()]
        || values("user") != ["git"]
        || values("identitiesonly") != ["yes"]
        || !values("stricthostkeychecking")
            .iter()
            .any(|value| value == "true" || value == "yes")
        || values("identityfile") != [expected_path]
    {
        return Err("Existing SSH rules add or override this profile's identity. Automatic assignment is disabled until those rules are reviewed.".into());
    }
    for key in ["proxycommand", "proxyjump"] {
        if values(key).iter().any(|value| value != "none") {
            return Err("An existing SSH proxy rule affects this alias; review it manually".into());
        }
    }
    Ok(())
}

pub fn verify(profile: &Profile) -> Result<Verification> {
    validate_profile(profile)?;
    // An isolated configuration prevents ProxyCommand/Match exec in user config.
    // Explicit IdentityFile and IdentitiesOnly keep verification profile-specific.
    let out = process::run(
        "ssh",
        &[
            "-F",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "IdentitiesOnly=yes",
            "-i",
            &profile.private_key_path,
            "-p",
            &profile.ssh_port.to_string(),
            &format!("git@{}", profile.host),
        ],
        None,
    )?;
    let text = format!("{}\n{}", out.stdout, out.stderr);
    Ok(profile
        .provider
        .verification(&text, &profile.username, &profile.host))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn profile() -> Profile {
        Profile {
            id: "fixture".into(),
            name: "Work".into(),
            username: "alice".into(),
            provider: crate::provider::GitProvider::Github,
            host: "github.com".into(),
            ssh_port: 22,
            git_name: "Alice".into(),
            git_email: "alice@example.com".into(),
            private_key_path: "/tmp/key".into(),
            public_key_path: "/tmp/key.pub".into(),
            ssh_alias: "github-work".into(),
            organization: String::new(),
            color: "mint".into(),
        }
    }
    #[test]
    fn preserves_existing_config() {
        let original = "# user notes\nHost example\n  HostName example.com\n";
        assert!(merge_config(original, &profile())
            .unwrap()
            .ends_with(original));
    }
    #[test]
    fn rejects_complex_or_conflicting_config() {
        for text in [
            "Include ~/.ssh/conf/*",
            "Match exec true",
            "Host github-work",
        ] {
            assert!(merge_config(text, &profile()).is_err());
        }
    }
    #[test]
    fn parses_success_and_mismatch() {
        let msg = "Hi alice! You've successfully authenticated, but GitHub does not provide shell access.";
        assert!(
            crate::provider::GitProvider::Github
                .verification(msg, "alice", "github.com")
                .success
        );
        assert!(
            !crate::provider::GitProvider::Github
                .verification(msg, "bob", "github.com")
                .success
        );
        assert!(
            !crate::provider::GitProvider::Github
                .verification("Permission denied", "alice", "github.com")
                .success
        );
    }
    #[test]
    fn rejects_injection() {
        let mut p = profile();
        p.ssh_alias = "github-a\nProxyCommand evil".into();
        assert!(validate_profile(&p).is_err());
        p = profile();
        p.private_key_path = "/tmp/%h".into();
        assert!(validate_profile(&p).is_err());
    }
    #[test]
    fn reapply_is_idempotent() {
        let p = profile();
        let once = merge_config("# original\n", &p).unwrap();
        assert_eq!(merge_config(&once, &p).unwrap(), once);
    }
    #[test]
    fn refuses_edited_managed_block() {
        let p = profile();
        let text = merge_config("", &p)
            .unwrap()
            .replace("HostName github.com", "HostName evil.example");
        assert!(merge_config(&text, &p).is_err());
    }
}
