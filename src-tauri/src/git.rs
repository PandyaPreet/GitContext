use crate::{
    model::{Profile, RepoStatus, Repository, Result},
    process, provider,
};
use std::path::{Path, PathBuf};

pub fn config(path: &Path, key: &str) -> String {
    process::git(&["config", "--get", key], Some(path)).unwrap_or_default()
}

pub fn register(value: &Path) -> Result<Repository> {
    let canonical = value
        .canonicalize()
        .map_err(|_| "Repository folder does not exist")?;
    let root = process::git(&["rev-parse", "--show-toplevel"], Some(&canonical))
        .map_err(|_| "This folder is not a working Git repository. Choose a cloned repository or initialize it with Git first.")?;
    let root = PathBuf::from(root)
        .canonicalize()
        .map_err(|_| "Cannot resolve repository root")?;
    Ok(Repository {
        id: uuid::Uuid::new_v4().to_string(),
        name: root
            .file_name()
            .ok_or("Invalid repository path")?
            .to_string_lossy()
            .into(),
        path: root.to_str().ok_or("Repository path must be UTF-8")?.into(),
        profile_id: None,
    })
}

pub fn config_path(repo: &Repository) -> Result<PathBuf> {
    let root = Path::new(&repo.path);
    let git_dir = process::git(&["rev-parse", "--absolute-git-dir"], Some(root))?;
    let common_dir = process::git(
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        Some(root),
    )?;
    if git_dir != common_dir {
        return Err("Linked worktrees share Git config. Register the primary worktree or configure a worktree-specific identity manually.".into());
    }
    if config(root, "extensions.worktreeConfig") == "true" {
        return Err("Per-worktree config is enabled; automatic assignment is not supported for this repository yet".into());
    }
    let output = process::git(
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "config",
        ],
        Some(root),
    )?;
    let path = PathBuf::from(output);
    if !path.is_absolute() {
        return Err("Git returned a relative config path; update Git".into());
    }
    Ok(path)
}

pub fn visible_remote(remote: &str) -> String {
    if remote.is_empty() || provider::parse_remote(remote).is_ok() {
        remote.into()
    } else {
        "Unsupported or credential-bearing remote (hidden)".into()
    }
}

pub fn inspect(
    repo: &Repository,
    profile: Option<&Profile>,
    profiles: &[Profile],
) -> Result<RepoStatus> {
    let root = Path::new(&repo.path);
    let author = config(root, "user.name");
    let email = config(root, "user.email");
    let remote = config(root, "remote.origin.url");
    let branch = process::git(&["branch", "--show-current"], Some(root))?;
    let status = process::git(
        &["status", "--porcelain", "--untracked-files=normal"],
        Some(root),
    )?;
    let ahead = process::git(
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        Some(root),
    )
    .unwrap_or_default();
    let identity_matches = profile
        .map(|p| {
            let remote_matches = remote.is_empty()
                || (remote.starts_with(&format!("git@{}:", p.ssh_alias))
                    && process::git(&["remote", "get-url", "origin"], Some(root))
                        .ok()
                        .as_deref()
                        == Some(remote.as_str())
                    && process::git(
                        &["remote", "get-url", "--push", "--all", "origin"],
                        Some(root),
                    )
                    .ok()
                    .as_deref()
                        == Some(remote.as_str()));
            author == p.git_name
                && email == p.git_email
                && remote_matches
                && config(root, "core.sshCommand").is_empty()
                && config(root, "ssh.variant").is_empty()
        })
        .unwrap_or(false);
    Ok(RepoStatus {
        id: repo.id.clone(),
        branch: if branch.is_empty() {
            "detached / unborn".into()
        } else {
            branch
        },
        git_name: author,
        git_email: email,
        remote: visible_remote(&remote),
        dirty: !status.is_empty(),
        changes: status.lines().count(),
        remote_info: provider::remote_info(&remote, profiles).ok(),
        ahead_behind: ahead,
        identity_matches,
    })
}

pub fn prepare_config(
    repo: &Repository,
    profile: &Profile,
    existing: &[u8],
    rewrite_remote: bool,
    profiles: &[Profile],
) -> Result<Vec<u8>> {
    use std::io::Write;
    let root = Path::new(&repo.path);
    if !config(root, "core.sshCommand").is_empty() || !config(root, "ssh.variant").is_empty() {
        return Err(
            "Custom Git SSH transport detected. Remove or review it before assigning a profile."
                .into(),
        );
    }
    let rewrites = process::run(
        "git",
        &[
            "config",
            "--get-regexp",
            "^url\\..*\\.(insteadof|pushinsteadof)$",
        ],
        Some(root),
    )?;
    if rewrites.code == 0 {
        return Err(
            "Git URL rewrite rules may override SSH identity. Review them before applying.".into(),
        );
    }
    let mut temp = tempfile::NamedTempFile::new().map_err(|_| "Cannot stage Git config")?;
    temp.write_all(existing)
        .map_err(|_| "Cannot stage Git config")?;
    let temp_path = temp.path().to_str().ok_or("Invalid temporary path")?;
    for (key, value) in [
        ("user.name", profile.git_name.as_str()),
        ("user.email", profile.git_email.as_str()),
    ] {
        process::git(
            &["config", "--file", temp_path, "--replace-all", key, value],
            None,
        )?;
    }
    let remote = config(root, "remote.origin.url");
    if !remote.is_empty() {
        let desired = provider::rewrite_remote(&remote, profile, profiles)?;
        let urls = process::git(&["config", "--get-all", "remote.origin.url"], Some(root))?;
        if urls.lines().count() != 1 || !config(root, "remote.origin.pushurl").is_empty() {
            return Err(
                "Multiple origin URLs or an explicit push URL require manual review".into(),
            );
        }
        if remote != desired && !rewrite_remote {
            return Err(
                "Origin must use the profile SSH alias. Enable remote rewriting in the preview."
                    .into(),
            );
        }
        process::git(
            &[
                "config",
                "--file",
                temp_path,
                "--replace-all",
                "remote.origin.url",
                &desired,
            ],
            None,
        )?;
    }
    std::fs::read(temp.path()).map_err(|_| "Cannot read staged Git config".into())
}
