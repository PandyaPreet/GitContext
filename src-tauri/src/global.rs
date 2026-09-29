//! Explicit global defaults. Never visits or rewrites repository configurations.
use crate::{
    model::{Profile, Result},
    process, ssh,
};
use std::{io::Write, path::Path};

const DISABLED: &str = "# Git Context global saved IdentityFile: ";

pub fn ssh_profile(profile: &Profile) -> Profile {
    let mut target = profile.clone();
    target.id = "global-default".into();
    target.ssh_alias = target.host.clone();
    target
}
pub fn stanza(profile: &Profile) -> String {
    let target = ssh_profile(profile);
    let text = ssh::stanza(&target);
    if target.ssh_port == 22 {
        text.replace("    User git\n", "    User git\n    Port 22\n")
    } else {
        text
    }
}

pub fn merge_ssh(existing: &str, previous: Option<&Profile>, profile: &Profile) -> Result<String> {
    // The shared parser rejects Includes/Match before any ssh -G invocation.
    for line in existing.lines() {
        let keyword = line.trim().split([' ', '\t', '=']).next().unwrap_or("");
        if keyword.eq_ignore_ascii_case("include") || keyword.eq_ignore_ascii_case("match") {
            return Err(
                "SSH Include/Match rules require manual review before profile activation.".into(),
            );
        }
    }
    let mut restored = existing.to_string();
    if let Some(previous) = previous {
        let old = stanza(previous);
        if restored.matches(&old).count() != 1 {
            return Err("The managed global SSH block changed or is missing. Restore its transaction or review the configuration before switching global defaults.".into());
        }
        restored = restored.replacen(&old, "", 1);
        restored = restored
            .split_inclusive('\n')
            .map(|line| line.strip_prefix(DISABLED).unwrap_or(line))
            .collect();
    } else if restored.contains("# Git Context begin global-default") || restored.contains(DISABLED)
    {
        return Err(
            "An untracked global SSH block exists. Review or restore its transaction first.".into(),
        );
    }
    let mut preserved = String::new();
    let mut exact_host = false;
    for line in restored.split_inclusive('\n') {
        let words: Vec<_> = line
            .trim()
            .split([' ', '\t', '='])
            .filter(|v| !v.is_empty())
            .collect();
        if words
            .first()
            .is_some_and(|w| w.eq_ignore_ascii_case("host"))
        {
            exact_host = words.len() == 2 && words[1].eq_ignore_ascii_case(&profile.host);
        }
        // IdentityFile is additive in OpenSSH. Preserve and temporarily disable
        // only this exact host's key lines. Wildcard/shared rules fail validation.
        if exact_host
            && words
                .first()
                .is_some_and(|w| w.eq_ignore_ascii_case("identityfile"))
        {
            preserved.push_str(DISABLED);
        }
        preserved.push_str(line);
    }
    Ok(format!("{}{preserved}", stanza(profile)))
}

pub fn prepare_git(existing: &[u8], profile: &Profile) -> Result<Vec<u8>> {
    let mut file = tempfile::NamedTempFile::new().map_err(|_| "Cannot stage global Git config")?;
    file.write_all(existing)
        .map_err(|_| "Cannot stage global Git config")?;
    let path = file.path().to_str().ok_or("Invalid config path")?;
    for (key, value) in [
        ("user.name", profile.git_name.as_str()),
        ("user.email", profile.git_email.as_str()),
    ] {
        process::git(
            &["config", "--file", path, "--replace-all", key, value],
            None,
        )?;
    }
    std::fs::read(file.path()).map_err(|_| "Cannot read staged Git config".into())
}

pub fn author(path: &Path) -> (String, String) {
    let get = |key| {
        process::git(
            &[
                "config",
                "--file",
                path.to_str().unwrap_or(""),
                "--includes",
                "--get",
                key,
            ],
            None,
        )
        .unwrap_or_default()
    };
    (get("user.name"), get("user.email"))
}
pub fn validate_git(path: &Path, profile: &Profile) -> Result<()> {
    let actual = author(path);
    if actual != (profile.git_name.clone(), profile.git_email.clone()) {
        return Err("A Git include overrides the global author. Review that rule before setting a global default.".into());
    }
    #[cfg(not(test))]
    for (key, expected) in [
        ("user.name", &profile.git_name),
        ("user.email", &profile.git_email),
    ] {
        if process::git(&["config", "--global", "--get", key], None)? != *expected {
            return Err("The effective global Git configuration differs from this profile.".into());
        }
    }
    Ok(())
}

// Retarget only exact app-owned alias blocks. Keep descriptors independently of
// saved profiles so deleting a profile cannot leave a forgotten active key.
pub fn retarget_aliases(
    text: &str,
    descriptors: &[Profile],
    target: &Profile,
    strict: bool,
) -> Result<(String, Vec<Profile>)> {
    let mut result = text.to_string();
    let mut installed = Vec::new();
    for old in descriptors {
        let marker = format!("# Git Context begin {}\n", old.id);
        if !result.contains(&marker) {
            if strict {
                return Err("A managed SSH alias was removed outside the app. Restore or review the SSH configuration before activating.".into());
            }
            continue;
        }
        let old_block = ssh::stanza(old);
        if result.matches(&old_block).count() != 1 {
            return Err("A managed SSH alias was edited externally. Review its configuration before activating another profile.".into());
        }
        let mut new = old.clone();
        if old.host == target.host && old.provider == target.provider {
            new.private_key_path = target.private_key_path.clone();
            new.public_key_path = target.public_key_path.clone();
            new.username = target.username.clone();
            new.ssh_port = target.ssh_port;
        }
        result = result.replacen(&old_block, &ssh::stanza(&new), 1);
        installed.push(new);
    }
    Ok((result, installed))
}

const ACTIVE_BEGIN: &str = "# Git Context active profile begin\n";
const ACTIVE_END: &str = "# Git Context active profile end\n";

pub fn ssh_command(transport: &Path) -> String {
    // Git uses its shell on both platforms. A separate config prevents additive
    // IdentityFile rules and existing multiplexed connections from bypassing activation.
    let path = transport.to_string_lossy().replace('\\', "/");
    format!("ssh -F '{}'", path.replace('\'', "'\\''"))
}

pub fn prepare_transport(existing: &str, profile: &Profile) -> Result<String> {
    ssh::validate_profile(profile)?;
    let key = profile.private_key_path.replace('\\', "/");
    let mut result = format!("# Git Context: only the activated key is eligible.\nHost *\n    IdentityFile \"{key}\"\n    IdentitiesOnly yes\n    CertificateFile none\n    PKCS11Provider none\n    PreferredAuthentications publickey\n    StrictHostKeyChecking yes\n    ControlMaster no\n    ControlPath none\n    ControlPersist no\n");
    // Copy routing and host trust only, never identities, authentication agents,
    // proxies, executable rules, includes or connection-sharing settings.
    for line in existing.lines() {
        let keyword = line
            .trim()
            .split([' ', '\t', '='])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(keyword.as_str(), "include" | "match") {
            return Err("SSH Include/Match rules require review before activation".into());
        }
        if matches!(
            keyword.as_str(),
            "host"
                | "hostname"
                | "user"
                | "port"
                | "hostkeyalias"
                | "userknownhostsfile"
                | "globalknownhostsfile"
        ) {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

pub fn prepare_activation_git(
    existing: &[u8],
    profile: &Profile,
    transport: &Path,
) -> Result<Vec<u8>> {
    let mut text = std::str::from_utf8(existing)
        .map_err(|_| "Global Git configuration is not UTF-8")?
        .to_string();
    if text.contains(ACTIVE_BEGIN) || text.contains(ACTIVE_END) {
        if text.matches(ACTIVE_BEGIN).count() != 1 || text.matches(ACTIVE_END).count() != 1 {
            return Err("The managed global Git block is malformed".into());
        }
        let start = text.find(ACTIVE_BEGIN).unwrap();
        let end = text.find(ACTIVE_END).unwrap() + ACTIVE_END.len();
        if end <= start {
            return Err("The managed global Git block is malformed".into());
        }
        text.replace_range(start..end, "");
    }
    let fragment = prepare_git(&[], profile)?;
    let mut file =
        tempfile::NamedTempFile::new().map_err(|_| "Cannot stage global SSH settings")?;
    file.write_all(&fragment)
        .map_err(|_| "Cannot stage global SSH settings")?;
    for (key, value) in [
        ("core.sshCommand", ssh_command(transport)),
        ("ssh.variant", "ssh".into()),
    ] {
        process::git(
            &[
                "config",
                "--file",
                file.path().to_str().ok_or("Invalid config path")?,
                "--replace-all",
                key,
                &value,
            ],
            None,
        )?;
    }
    let fragment =
        std::fs::read_to_string(file.path()).map_err(|_| "Cannot read global SSH settings")?;
    // Last wins, including over earlier global includes. Preserve their files.
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(ACTIVE_BEGIN);
    text.push_str(&fragment);
    text.push_str(ACTIVE_END);
    Ok(text.into_bytes())
}

pub fn validate_activation_git(path: &Path, transport: &Path) -> Result<()> {
    for (key, expected) in [
        ("core.sshCommand", ssh_command(transport)),
        ("ssh.variant", "ssh".into()),
    ] {
        let actual = process::git(
            &[
                "config",
                "--file",
                path.to_str().ok_or("Invalid config path")?,
                "--includes",
                "--get",
                key,
            ],
            None,
        )?;
        if actual != expected {
            return Err("Global Git SSH settings differ from the activated profile".into());
        }
    }
    Ok(())
}
