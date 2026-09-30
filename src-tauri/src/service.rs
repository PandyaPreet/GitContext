use crate::{
    detection, git, global,
    model::*,
    provider, ssh,
    storage::{self, FileChange, Journal},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct Plan {
    pub view: PlanView,
    pub files: Vec<FileChange>,
    pub state_before: AppData,
    pub global: bool,
    pub activation_aliases: Option<Vec<Profile>>,
    pub migrated_repositories: Vec<String>,
}
pub struct Service {
    pub root: PathBuf,
    pub data: AppData,
    pub plans: HashMap<String, Plan>,
    ssh_config_path: PathBuf,
    global_git_path: PathBuf,
}

impl Service {
    pub fn new(root: PathBuf) -> Result<Self> {
        let data = storage::load(&root)?;
        Ok(Self {
            root,
            data,
            plans: HashMap::new(),
            ssh_config_path: detection::home()?.join(".ssh/config"),
            global_git_path: detection::home()?.join(".gitconfig"),
        })
    }
    pub fn commit(&mut self, next: AppData) -> Result<AppData> {
        storage::save(&self.root, &next)?;
        self.data = next;
        self.plans.clear();
        Ok(self.data.clone())
    }
    pub fn profile(&self, id: &str) -> Result<&Profile> {
        self.data
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("Profile not found".into())
    }
    pub fn repository(&self, id: &str) -> Result<&Repository> {
        self.data
            .repositories
            .iter()
            .find(|r| r.id == id)
            .ok_or("Repository not found".into())
    }
    pub fn validate_ssh(&self, profile_id: &str) -> Result<()> {
        let profile = self.profile(profile_id)?;
        let bytes = storage::read_optional(&self.ssh_config_path)?
            .ok_or("SSH alias config is missing; reapply the profile")?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "Invalid SSH config")?;
        // Validates user rules without executing Match exec or following Includes.
        ssh::merge_config(text, profile)?;
        if !text.contains(&format!("# Git Context begin {}", profile.id)) {
            return Err("Managed SSH alias is missing; reapply the profile".into());
        }
        ssh::validate_config(text, profile, &self.root)?;
        ssh::validate_installed(profile, &self.ssh_config_path)
    }

    pub fn health(&self) -> HealthReport {
        let mut checks = Vec::new();
        let mut check = |label: &str, result: Result<()>, good: &str| {
            checks.push(HealthCheck {
                label: label.into(),
                ok: result.is_ok(),
                detail: result.err().unwrap_or_else(|| good.into()),
            });
        };
        let profile = self
            .data
            .global_profile_id
            .as_deref()
            .filter(|_| self.data.single_profile_mode)
            .and_then(|id| self.profile(id).ok());
        if let Some(p) = profile {
            check(
                "SSH key",
                (|| {
                    ssh::validate_profile(p)?;
                    if !Path::new(&p.private_key_path).is_file() {
                        return Err("Selected private key is missing. Restore it or select another profile.".into());
                    }
                    let key = detection::inspect_key(Path::new(&p.public_key_path), "")?;
                    if key.private_path != p.private_key_path {
                        return Err("Public and private key paths do not match. Edit the profile's key reference.".into());
                    }
                    Ok(())
                })(),
                "Selected key files are available.",
            );
            check(
                "Git author",
                global::validate_git(&self.global_git_path, p),
                "Global name and email match this profile.",
            );
            let transport = self.root.join("active-ssh-config");
            check(
                "Git SSH command",
                global::validate_activation_git(&self.global_git_path, &transport),
                "Git uses the managed SSH transport.",
            );
            // Never execute an externally edited SSH config: compare against our
            // allowlisted transport before asking OpenSSH to resolve anything.
            let transport_check = (|| {
                let source = std::fs::read_to_string(&self.ssh_config_path)
                    .map_err(|_| "SSH configuration is missing or unreadable")?;
                let expected = global::prepare_transport(&source, p)?;
                let actual = std::fs::read_to_string(&transport)
                    .map_err(|_| "Managed SSH transport is missing or unreadable")?;
                if actual != expected {
                    return Err("SSH configuration changed outside Git Context. Review the files, then reapply the profile.".into());
                }
                ssh::validate_config(&expected, &global::ssh_profile(p), &self.root)
            })();
            check(
                "SSH configuration",
                transport_check,
                "Managed transport matches the selected key and host routing.",
            );
        } else {
            check(
                "Active profile",
                Err("Activate a profile to check its configuration.".into()),
                "",
            );
        }
        HealthReport {
            profile_id: profile.map(|p| p.id.clone()),
            checks,
        }
    }

    pub fn set_color(&mut self, id: &str, color: &str) -> Result<AppData> {
        if !["mint", "blue", "violet", "amber"].contains(&color) {
            return Err("Unknown profile colour".into());
        }
        self.profile(id)?;
        let mut next = self.data.clone();
        next.profiles.iter_mut().find(|p| p.id == id).unwrap().color = color.into();
        self.commit(next)
    }

    pub fn create_profile(&mut self, mut profile: Profile) -> Result<AppData> {
        profile.id = uuid::Uuid::new_v4().to_string();
        profile.host = profile.host.to_ascii_lowercase();
        let key = detection::inspect_key(&detection::expand(&profile.public_key_path)?, "")?;
        profile.private_key_path = key.private_path;
        profile.public_key_path = key.public_path;
        ssh::validate_profile(&profile)?;
        if self
            .data
            .profiles
            .iter()
            .any(|p| p.ssh_alias.eq_ignore_ascii_case(&profile.ssh_alias))
        {
            return Err("That SSH alias belongs to another profile".into());
        }
        let mut next = self.data.clone();
        if next.active_profile_id.is_none() {
            next.active_profile_id = Some(profile.id.clone());
        }
        next.profiles.push(profile);
        self.commit(next)
    }
    pub fn update_profile(&mut self, mut profile: Profile) -> Result<AppData> {
        let previous = self.profile(&profile.id)?.clone();
        if self.data.global_profile_id.as_deref() == Some(&profile.id) {
            return Err("Duplicate this global profile before changing its identity.".into());
        }
        if self
            .data
            .repositories
            .iter()
            .any(|r| r.profile_id.as_deref() == Some(&profile.id))
        {
            return Err("This profile is assigned to repositories. Duplicate it, then review and apply the new identity to each repository.".into());
        }
        profile.host = profile.host.to_ascii_lowercase();
        let key = detection::inspect_key(&detection::expand(&profile.public_key_path)?, "")?;
        profile.private_key_path = key.private_path;
        profile.public_key_path = key.public_path;
        ssh::validate_profile(&profile)?;
        if self
            .data
            .profiles
            .iter()
            .any(|p| p.id != profile.id && p.ssh_alias.eq_ignore_ascii_case(&profile.ssh_alias))
        {
            return Err("That SSH alias belongs to another profile".into());
        }
        let existing = storage::read_optional(&self.ssh_config_path)?.unwrap_or_default();
        if String::from_utf8_lossy(&existing)
            .contains(&format!("# Git Context begin {}", previous.id))
        {
            return Err("This profile already owns an SSH configuration block. Duplicate it to change identity without silently rewriting that block.".into());
        }
        let mut next = self.data.clone();
        let id = profile.id.clone();
        *next
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or("Profile not found")? = profile;
        self.commit(next)
    }
    pub fn rename_profile(&mut self, id: &str, name: &str) -> Result<AppData> {
        self.profile(id)?;
        Self::validate_label(name)?;
        let mut next = self.data.clone();
        next.profiles.iter_mut().find(|p| p.id == id).unwrap().name = name.trim().into();
        self.commit(next)
    }
    fn validate_label(name: &str) -> Result<()> {
        if name.trim().is_empty() || name.len() > 200 || name.chars().any(char::is_control) {
            return Err("Enter a name of 1–200 characters without control characters.".into());
        }
        Ok(())
    }
    pub fn remove_profile(&mut self, id: &str) -> Result<AppData> {
        self.profile(id)?;
        if self.data.global_profile_id.as_deref() == Some(id) {
            return Err("This profile is the global default. Set another global default or restore its configuration transaction first.".into());
        }
        let repositories: Vec<_> = self
            .data
            .repositories
            .iter()
            .filter(|r| r.profile_id.as_deref() == Some(id))
            .map(|r| r.name.as_str())
            .collect();
        if !repositories.is_empty() {
            return Err(format!("This profile is assigned to: {}. Activate a profile for this provider to retire those old assignments first. Configuration changes require a preview.", repositories.join(", ")));
        }
        let mut next = self.data.clone();
        next.profiles.retain(|p| p.id != id);
        if next.active_profile_id.as_deref() == Some(id) {
            next.active_profile_id = next.profiles.first().map(|p| p.id.clone());
        }
        self.commit(next)
    }
    pub fn update_repository(&mut self, id: &str, name: &str, path: &str) -> Result<AppData> {
        Self::validate_label(name)?;
        let previous = self.repository(id)?.clone();
        let mut replacement = if path == previous.path {
            previous.clone()
        } else {
            let mut repo = git::register(&detection::expand(path)?)?;
            if self
                .data
                .repositories
                .iter()
                .any(|r| r.id != id && r.path == repo.path)
            {
                return Err("This repository is already registered".into());
            }
            repo.id = id.into();
            if repo.path == previous.path {
                repo.profile_id = previous.profile_id;
            }
            repo
        };
        replacement.name = name.trim().into();
        let mut next = self.data.clone();
        *next.repositories.iter_mut().find(|r| r.id == id).unwrap() = replacement;
        self.commit(next)
    }
    pub fn remove_repository(&mut self, id: &str) -> Result<AppData> {
        self.repository(id)?;
        let mut next = self.data.clone();
        next.repositories.retain(|r| r.id != id);
        self.commit(next)
    }
    pub fn register_key(&mut self, path: &str) -> Result<AppData> {
        let key = detection::inspect_key(&detection::expand(path)?, "")?;
        let mut next = self.data.clone();
        next.hidden_key_paths.retain(|p| p != &key.public_path);
        if !next.imported_key_paths.contains(&key.public_path) {
            next.imported_key_paths.push(key.public_path);
        }
        self.commit(next)
    }
    pub fn remove_key_reference(&mut self, path: &str) -> Result<AppData> {
        if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
            return Err("Invalid key reference".into());
        }
        if self.data.profiles.iter().any(|p| p.public_key_path == path) {
            return Err("This key is assigned to a profile. Change that profile's key or remove the profile first.".into());
        }
        let mut next = self.data.clone();
        next.imported_key_paths.retain(|p| p != path);
        if !next.hidden_key_paths.iter().any(|p| p == path) {
            next.hidden_key_paths.push(path.into());
        }
        self.commit(next)
    }
    pub fn select_profile(&mut self, id: &str) -> Result<AppData> {
        self.profile(id)?;
        let mut next = self.data.clone();
        next.active_profile_id = Some(id.into());
        self.commit(next)
    }
    pub fn register_repository(&mut self, path: &str) -> Result<AppData> {
        let repo = git::register(&detection::expand(path)?)?;
        if self.data.repositories.iter().any(|r| r.path == repo.path) {
            return Err("This repository is already registered".into());
        }
        let mut next = self.data.clone();
        next.repositories.push(repo);
        self.commit(next)
    }
    pub fn plan_assignment(
        &mut self,
        repository_id: &str,
        profile_id: &str,
        rewrite_remote: bool,
    ) -> Result<PlanView> {
        let repo = self.repository(repository_id)?.clone();
        let profile = self.profile(profile_id)?.clone();
        ssh::validate_profile(&profile)?;
        detection::inspect_key(Path::new(&profile.public_key_path), "")?;
        let git_path = git::config_path(&repo)?;
        let git_before = storage::read_optional(&git_path)?.ok_or("Missing repository config")?;
        let git_after = git::prepare_config(
            &repo,
            &profile,
            &git_before,
            rewrite_remote,
            &self.data.profiles,
        )?;
        let ssh_path = self.ssh_config_path.clone();
        let ssh_before = storage::read_optional(&ssh_path)?;
        let ssh_text = std::str::from_utf8(ssh_before.as_deref().unwrap_or_default())
            .map_err(|_| "SSH configuration is not UTF-8")?;
        let ssh_after = ssh::merge_config(ssh_text, &profile)?;
        ssh::validate_config(&ssh_after, &profile, &self.root)?;
        let status = git::inspect(&repo, None, &self.data.profiles)?;
        let new_remote = if status.remote.is_empty() {
            String::new()
        } else {
            provider::rewrite_remote(&status.remote, &profile, &self.data.profiles)?
        };
        let view = PlanView {
            id: uuid::Uuid::new_v4().to_string(),
            repository_id: repo.id.clone(),
            profile_id: profile.id.clone(),
            ssh_stanza: ssh::stanza(&profile),
            changes: vec![
                Change {
                    label: "Repository Git author".into(),
                    before: status.git_name,
                    after: profile.git_name.clone(),
                },
                Change {
                    label: "Repository Git email".into(),
                    before: status.git_email,
                    after: profile.git_email.clone(),
                },
                Change {
                    label: "Origin remote".into(),
                    before: status.remote,
                    after: new_remote,
                },
                Change {
                    label: "SSH alias".into(),
                    before: "Existing user configuration preserved".into(),
                    after: format!(
                        "{} → {}:{}",
                        profile.ssh_alias, profile.host, profile.ssh_port
                    ),
                },
            ],
        };
        self.plans.clear();
        self.plans.insert(
            view.id.clone(),
            Plan {
                view: view.clone(),
                files: vec![
                    FileChange {
                        path: ssh_path,
                        before: ssh_before,
                        after: ssh_after.into_bytes(),
                    },
                    FileChange {
                        path: git_path,
                        before: Some(git_before),
                        after: git_after,
                    },
                ],
                state_before: self.data.clone(),
                global: false,
                activation_aliases: None,
                migrated_repositories: vec![],
            },
        );
        Ok(view)
    }
    pub fn plan_global(&mut self, profile_id: &str) -> Result<PlanView> {
        #[cfg(not(test))]
        if std::env::var_os("GIT_CONFIG_GLOBAL").is_some() {
            return Err("GIT_CONFIG_GLOBAL overrides the default file. Launch without that override before applying global defaults.".into());
        }
        let profile = self.profile(profile_id)?.clone();
        ssh::validate_profile(&profile)?;
        detection::inspect_key(Path::new(&profile.public_key_path), "")?;
        let previous = self
            .data
            .global_profile_id
            .as_ref()
            .map(|id| self.profile(id))
            .transpose()?;
        let ssh_before = storage::read_optional(&self.ssh_config_path)?;
        let ssh_text = std::str::from_utf8(ssh_before.as_deref().unwrap_or_default())
            .map_err(|_| "SSH configuration is not UTF-8")?;
        let ssh_after = global::merge_ssh(ssh_text, previous, &profile)?;
        ssh::validate_config(&ssh_after, &global::ssh_profile(&profile), &self.root)?;
        let git_before = storage::read_optional(&self.global_git_path)?;
        let git_after = global::prepare_git(git_before.as_deref().unwrap_or_default(), &profile)?;
        let (name, email) = global::author(&self.global_git_path);
        let view = PlanView { id: uuid::Uuid::new_v4().to_string(), repository_id: String::new(), profile_id: profile.id.clone(), ssh_stanza: global::stanza(&profile), changes: vec![
            Change { label: "Global Git author".into(), before: name, after: profile.git_name.clone() },
            Change { label: "Global Git email".into(), before: email, after: profile.git_email.clone() },
            Change { label: "Default SSH identity".into(), before: previous.map(|p| format!("{} · {}", p.host, p.name)).unwrap_or("Existing host configuration".into()), after: format!("{}:{} · {}", profile.host, profile.ssh_port, profile.private_key_path) },
            Change { label: "Scope".into(), before: "Repository overrides retained".into(), after: "Git author: all repositories without overrides. SSH: direct connections to this provider host. HTTPS credentials, other hosts and repository aliases stay unchanged.".into() },
        ]};
        self.plans.clear();
        self.plans.insert(
            view.id.clone(),
            Plan {
                view: view.clone(),
                files: vec![
                    FileChange {
                        path: self.ssh_config_path.clone(),
                        before: ssh_before,
                        after: ssh_after.into_bytes(),
                    },
                    FileChange {
                        path: self.global_git_path.clone(),
                        before: git_before,
                        after: git_after,
                    },
                ],
                state_before: self.data.clone(),
                global: true,
                activation_aliases: None,
                migrated_repositories: vec![],
            },
        );
        Ok(view)
    }
    pub fn plan_activation(&mut self, profile_id: &str) -> Result<PlanView> {
        let target = self.profile(profile_id)?.clone();
        let original = storage::read_optional(&self.ssh_config_path)?;
        let text = std::str::from_utf8(original.as_deref().unwrap_or_default())
            .map_err(|_| "Invalid SSH config")?;
        let descriptors = if self.data.single_profile_mode {
            &self.data.managed_aliases
        } else {
            &self.data.profiles
        };
        let (retargeted, aliases) =
            global::retarget_aliases(text, descriptors, &target, self.data.single_profile_mode)?;
        // Build global fields/backup first, then replace the candidate SSH bytes.
        let view = self.plan_global(profile_id)?;
        let previous = self
            .data
            .global_profile_id
            .as_ref()
            .map(|id| self.profile(id))
            .transpose()?;
        let ssh_after = global::merge_ssh(&retargeted, previous, &target)?;
        ssh::validate_config(&ssh_after, &global::ssh_profile(&target), &self.root)?;
        for alias in &aliases {
            ssh::validate_config(&ssh_after, alias, &self.root)?;
        }
        // Global activation never inspects or rewrites repository files.
        let migrated: Vec<String> = self
            .data
            .repositories
            .iter()
            .filter(|repo| {
                repo.profile_id
                    .as_ref()
                    .and_then(|id| self.profile(id).ok())
                    .is_some_and(|p| p.host == target.host && p.provider == target.provider)
            })
            .map(|repo| repo.id.clone())
            .collect();
        let git_before = storage::read_optional(&self.global_git_path)?;
        let transport_path = self.root.join("active-ssh-config");
        let transport = global::prepare_transport(&ssh_after, &target)?;
        ssh::validate_config(&transport, &global::ssh_profile(&target), &self.root)?;
        let transport_before = storage::read_optional(&transport_path)?;
        let git_after = global::prepare_activation_git(
            git_before.as_deref().unwrap_or_default(),
            &target,
            &transport_path,
        )?;
        let plan = self
            .plans
            .get_mut(&view.id)
            .ok_or("Activation preview missing")?;
        plan.files[0].after = ssh_after.into_bytes();
        plan.files[1].after = git_after;
        plan.files.push(FileChange {
            path: transport_path.clone(),
            before: transport_before,
            after: transport.into_bytes(),
        });
        plan.activation_aliases = Some(aliases.clone());
        plan.migrated_repositories = migrated;
        plan.view.changes.retain(|c| c.label != "Scope");
        plan.view.changes.push(Change {
            label: "Existing app SSH aliases".into(),
            before: "Previously assigned keys".into(),
            after: aliases
                .iter()
                .filter(|a| a.host == target.host)
                .map(|a| format!("{} → {}", a.ssh_alias, target.name))
                .collect::<Vec<_>>()
                .join("\n"),
        });
        plan.view.changes.push(Change {
            label: "Global Git SSH command".into(),
            before: "Existing global SSH command (including inherited settings)".into(),
            after: global::ssh_command(&transport_path),
        });
        plan.view.changes.push(Change {
            label: "Scope".into(), before: "Global defaults".into(),
            after: "Set global Git author and SSH key. Local repository overrides and HTTPS credentials remain independent; no repository files are inspected or changed.".into(),
        });
        Ok(plan.view.clone())
    }
    pub fn apply(&mut self, plan_id: &str) -> Result<String> {
        let plan = self
            .plans
            .remove(plan_id)
            .ok_or("Preview expired; review the changes again")?;
        let mut _locks = vec![];
        for change in &plan.files {
            if let Some(parent) = change.path.parent() {
                if !parent.exists() {
                    storage::private_dir(parent)?;
                }
            }
            _locks.push(storage::ConfigLock::acquire(&change.path)?);
        }
        for change in &plan.files {
            if storage::read_optional(&change.path)? != change.before {
                return Err("Configuration changed since preview. Review a fresh plan.".into());
            }
        }
        let mut next = self.data.clone();
        if plan.global {
            next.global_profile_id = Some(plan.view.profile_id.clone());
            if let Some(aliases) = &plan.activation_aliases {
                next.single_profile_mode = true;
                next.managed_aliases = aliases.clone();
                next.active_profile_id = Some(plan.view.profile_id.clone());
                for repo in &mut next.repositories {
                    if plan.migrated_repositories.contains(&repo.id) {
                        repo.profile_id = None;
                    }
                }
            }
        } else {
            next.repositories
                .iter_mut()
                .find(|r| r.id == plan.view.repository_id)
                .ok_or("Repository missing")?
                .profile_id = Some(plan.view.profile_id.clone());
        }
        let mut journal = Journal {
            id: plan.view.id,
            files: plan.files,
            state_before: plan.state_before,
            state_after: next.clone(),
            status: "pending".into(),
        };
        storage::save_journal(&self.root, &journal)?;
        let result: Result<()> = (|| {
            for change in &journal.files {
                if let Some(parent) = change.path.parent() {
                    if !parent.exists() {
                        storage::private_dir(parent)?;
                    }
                }
                // Recheck immediately before replacement, in addition to the preflight.
                if storage::read_optional(&change.path)? != change.before {
                    return Err("Configuration changed during apply".into());
                }
                storage::atomic_write(&change.path, &change.after)?;
            }
            if plan.global {
                let profile = self.profile(&plan.view.profile_id)?;
                global::validate_git(&self.global_git_path, profile)?;
                ssh::validate_installed(&global::ssh_profile(profile), &self.ssh_config_path)?;
                if let Some(aliases) = &plan.activation_aliases {
                    for alias in aliases {
                        ssh::validate_installed(alias, &self.ssh_config_path)?;
                    }
                    global::validate_activation_git(
                        &self.global_git_path,
                        &self.root.join("active-ssh-config"),
                    )?;
                    ssh::validate_installed(
                        &global::ssh_profile(profile),
                        &self.root.join("active-ssh-config"),
                    )?;
                }
            } else {
                let repo = self.repository(&plan.view.repository_id)?;
                if !git::inspect(
                    repo,
                    Some(self.profile(&plan.view.profile_id)?),
                    &self.data.profiles,
                )?
                .identity_matches
                {
                    return Err("Effective Git identity differs from the requested profile".into());
                }
                self.validate_ssh(&plan.view.profile_id)?;
            }
            storage::save(&self.root, &next)?;
            Ok(())
        })();
        if let Err(error) = result {
            let mut conflict = false;
            for change in journal.files.iter().rev() {
                match storage::read_optional(&change.path) {
                    Ok(Some(current)) if current == change.after => {
                        if storage::restore(change).is_err() {
                            conflict = true;
                        }
                    }
                    Ok(current) if current == change.before => {}
                    _ => conflict = true,
                }
            }
            journal.status = if conflict {
                "recovery-required"
            } else {
                "rolled-back"
            }
            .into();
            let _ = storage::save_journal(&self.root, &journal);
            return Err(format!(
                "{error}. {}",
                if conflict {
                    "Recovery needs review; backup retained."
                } else {
                    "Configuration changes rolled back."
                }
            ));
        }
        self.data = next;
        journal.status = "applied".into();
        storage::save_journal(&self.root, &journal)?;
        Ok(journal.id)
    }

    pub fn journals(&self) -> Result<Vec<(String, String)>> {
        let mut result = vec![];
        if let Ok(entries) = std::fs::read_dir(self.root.join("backups")) {
            for entry in entries.flatten() {
                let bytes = storage::read_optional(&entry.path())?.unwrap_or_default();
                if let Ok(j) = serde_json::from_slice::<Journal>(&bytes) {
                    result.push((j.id, j.status));
                }
            }
        }
        Ok(result)
    }
    pub fn undo(&mut self, id: &str) -> Result<AppData> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid transaction ID")?;
        let bytes = storage::read_optional(&self.root.join("backups").join(format!("{id}.json")))?
            .ok_or("Backup not found")?;
        let mut journal: Journal = serde_json::from_slice(&bytes).map_err(|_| "Invalid backup")?;
        if !["applied", "pending", "recovery-required"].contains(&journal.status.as_str()) {
            return Err("Transaction was already rolled back".into());
        }
        let mut _locks = vec![];
        for change in &journal.files {
            _locks.push(storage::ConfigLock::acquire(&change.path)?);
        }
        // Preserve profiles/settings added after an assignment; only restore that assignment.
        let mut next = self.data.clone();
        if journal.state_before.single_profile_mode != journal.state_after.single_profile_mode
            || serde_json::to_string(&journal.state_before.managed_aliases).ok()
                != serde_json::to_string(&journal.state_after.managed_aliases).ok()
        {
            if next.global_profile_id != journal.state_after.global_profile_id {
                return Err("Another profile was activated after this transaction".into());
            }
            next.single_profile_mode = journal.state_before.single_profile_mode;
            next.managed_aliases = journal.state_before.managed_aliases.clone();
            next.active_profile_id = journal.state_before.active_profile_id.clone();
        }
        if journal.state_before.global_profile_id != journal.state_after.global_profile_id {
            if next.global_profile_id != journal.state_after.global_profile_id
                && next.global_profile_id != journal.state_before.global_profile_id
            {
                return Err("Global default changed since this transaction".into());
            }
            next.global_profile_id = journal.state_before.global_profile_id.clone();
        }
        for after in &journal.state_after.repositories {
            let before = journal
                .state_before
                .repositories
                .iter()
                .find(|r| r.id == after.id);
            if before.map(|r| &r.profile_id) != Some(&after.profile_id) {
                let current = next
                    .repositories
                    .iter_mut()
                    .find(|r| r.id == after.id)
                    .ok_or("Repository removed after transaction")?;
                if current.profile_id != after.profile_id
                    && Some(&current.profile_id) != before.map(|r| &r.profile_id)
                {
                    return Err("Repository assignment changed since transaction".into());
                }
                current.profile_id = before.and_then(|r| r.profile_id.clone());
            }
        }
        if next
            .repositories
            .iter()
            .filter_map(|r| r.profile_id.as_ref())
            .chain(next.global_profile_id.iter())
            .any(|id| !next.profiles.iter().any(|p| &p.id == id))
        {
            return Err("This backup references a removed profile. Review the saved configuration manually; no files were changed.".into());
        }
        for change in &journal.files {
            let current = storage::read_optional(&change.path)?;
            if current != Some(change.after.clone()) && current != change.before {
                return Err("Configuration changed since assignment; rollback refused to protect newer edits".into());
            }
        }
        for change in journal.files.iter().rev() {
            if storage::read_optional(&change.path)? == Some(change.after.clone()) {
                storage::restore(change)?;
            }
        }
        storage::save(&self.root, &next)?;
        self.data = next;
        journal.status = "rolled-back".into();
        storage::save_journal(&self.root, &journal)?;
        self.plans.clear();
        Ok(self.data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, Service, String, String) {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("app");
        let mut service = Service::new(app).unwrap();
        service.ssh_config_path = temp.path().join("ssh/config");
        service.global_git_path = temp.path().join("global.gitconfig");
        storage::private_dir(service.ssh_config_path.parent().unwrap()).unwrap();
        fs::write(
            &service.ssh_config_path,
            "# retain my comment\nHost internal\n  HostName internal.example\n",
        )
        .unwrap();
        let repo = temp.path().join("repo with spaces");
        fs::create_dir(&repo).unwrap();
        process::git(&["init", "--quiet"], Some(&repo)).unwrap();
        process::git(&["config", "user.name", "Previous"], Some(&repo)).unwrap();
        process::git(
            &["config", "user.email", "previous@example.com"],
            Some(&repo),
        )
        .unwrap();
        process::git(
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/acme/project.git",
            ],
            Some(&repo),
        )
        .unwrap();
        let key = temp.path().join("fixture key");
        assert_eq!(
            process::run(
                "ssh-keygen",
                &["-q", "-t", "ed25519", "-N", "", "-f", key.to_str().unwrap()],
                None
            )
            .unwrap()
            .code,
            0
        );
        let profile = Profile {
            id: String::new(),
            name: "Work".into(),
            username: "alice".into(),
            provider: crate::provider::GitProvider::Github,
            host: "github.com".into(),
            ssh_port: 22,
            git_name: "Alice".into(),
            git_email: "alice@acme.com".into(),
            private_key_path: String::new(),
            public_key_path: format!("{}.pub", key.display()),
            ssh_alias: "github-work".into(),
            organization: "Acme".into(),
            color: "mint".into(),
        };
        service.create_profile(profile).unwrap();
        service.register_repository(repo.to_str().unwrap()).unwrap();
        let pid = service.data.profiles[0].id.clone();
        let rid = service.data.repositories[0].id.clone();
        (temp, service, pid, rid)
    }

    #[test]
    fn assignment_roundtrip_preserves_git_and_ssh_bytes() {
        let (_temp, mut service, pid, rid) = fixture();
        let git_path = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let git_before = fs::read(&git_path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        assert_eq!(
            fs::read(&git_path).unwrap(),
            git_before,
            "preview must be read-only"
        );
        let id = service.apply(&plan.id).unwrap();
        assert!(
            git::inspect(
                service.repository(&rid).unwrap(),
                Some(service.profile(&pid).unwrap()),
                &service.data.profiles,
            )
            .unwrap()
            .identity_matches
        );
        assert!(fs::read_to_string(&service.ssh_config_path)
            .unwrap()
            .ends_with(std::str::from_utf8(&ssh_before).unwrap()));
        service.validate_ssh(&pid).unwrap();
        service.undo(&id).unwrap();
        assert_eq!(fs::read(git_path).unwrap(), git_before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
        assert!(service.repository(&rid).unwrap().profile_id.is_none());
    }
    #[test]
    fn stale_preview_does_not_write_any_file() {
        let (_temp, mut service, pid, rid) = fixture();
        let path = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let before = fs::read(&path).unwrap();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        fs::write(&service.ssh_config_path, "# external edit\n").unwrap();
        assert!(service
            .apply(&plan.id)
            .unwrap_err()
            .contains("changed since preview"));
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(
            fs::read_to_string(&service.ssh_config_path).unwrap(),
            "# external edit\n"
        );
    }
    #[test]
    fn effective_identity_failure_rolls_back_both_files() {
        let (temp, mut service, pid, rid) = fixture();
        let override_path = temp.path().join("override.gitconfig");
        fs::write(
            &override_path,
            "[user]\n    email = unexpected@example.com\n",
        )
        .unwrap();
        let repository = service.repository(&rid).unwrap().clone();
        process::git(
            &["config", "include.path", override_path.to_str().unwrap()],
            Some(Path::new(&repository.path)),
        )
        .unwrap();
        let git_path = git::config_path(&repository).unwrap();
        let git_before = fs::read(&git_path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        assert!(service.apply(&plan.id).unwrap_err().contains("rolled back"));
        assert_eq!(fs::read(git_path).unwrap(), git_before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
        assert!(service.repository(&rid).unwrap().profile_id.is_none());
    }
    #[test]
    fn wildcard_extra_ssh_key_is_rejected_before_writes() {
        let (_temp, mut service, pid, rid) = fixture();
        fs::write(
            &service.ssh_config_path,
            "Host *\n    IdentityFile ~/.ssh/unrelated\n",
        )
        .unwrap();
        assert!(service
            .plan_assignment(&rid, &pid, true)
            .err()
            .unwrap()
            .contains("override this profile"));
    }
    #[test]
    fn detects_push_destination_drift() {
        let (_temp, mut service, pid, rid) = fixture();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&plan.id).unwrap();
        let repository = service.repository(&rid).unwrap();
        process::git(
            &[
                "config",
                "remote.origin.pushurl",
                "https://github.com/other/repository.git",
            ],
            Some(Path::new(&repository.path)),
        )
        .unwrap();
        assert!(
            !git::inspect(
                repository,
                Some(service.profile(&pid).unwrap()),
                &service.data.profiles
            )
            .unwrap()
            .identity_matches
        );
    }
    #[test]
    fn rollback_protects_newer_edits() {
        let (_temp, mut service, pid, rid) = fixture();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        let id = service.apply(&plan.id).unwrap();
        fs::write(&service.ssh_config_path, "# new user edits\n").unwrap();
        assert!(service.undo(&id).is_err());
        assert_eq!(
            fs::read_to_string(&service.ssh_config_path).unwrap(),
            "# new user edits\n"
        );
    }
    #[test]
    fn selection_does_not_change_git_or_ssh() {
        let (_temp, mut service, pid, rid) = fixture();
        let path = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let before = fs::read(&path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        service.select_profile(&pid).unwrap();
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
    }
    #[test]
    fn respects_external_git_config_lock() {
        let (_temp, mut service, pid, rid) = fixture();
        let path = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        let _lock = storage::ConfigLock::acquire(&path).unwrap();
        let before = fs::read(&service.ssh_config_path).unwrap();
        assert!(service.apply(&plan.id).unwrap_err().contains("locked"));
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), before);
    }
    #[test]
    fn duplicate_repository_and_alias_are_rejected() {
        let (_temp, mut service, pid, rid) = fixture();
        let path = service.repository(&rid).unwrap().path.clone();
        assert!(service.register_repository(&path).is_err());
        assert!(service
            .create_profile(service.profile(&pid).unwrap().clone())
            .is_err());
    }
    #[test]
    fn application_can_recover_pending_journal_after_restart() {
        let (_temp, mut service, pid, rid) = fixture();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        let plan = service.plans.remove(&plan.id).unwrap();
        let journal = Journal {
            id: plan.view.id.clone(),
            files: plan.files,
            state_before: service.data.clone(),
            state_after: service.data.clone(),
            status: "pending".into(),
        };
        storage::save_journal(&service.root, &journal).unwrap();
        storage::atomic_write(&journal.files[0].path, &journal.files[0].after).unwrap();
        let mut restarted = Service::new(service.root.clone()).unwrap();
        restarted.undo(&journal.id).unwrap();
        assert_eq!(
            storage::read_optional(&journal.files[0].path).unwrap(),
            journal.files[0].before
        );
    }
    #[test]
    fn gitlab_subgroup_assignment_and_rollback_preserve_destination() {
        let (_temp, mut service, pid, rid) = fixture();
        let mut profile = service.profile(&pid).unwrap().clone();
        profile.provider = crate::provider::GitProvider::Gitlab;
        profile.host = "gitlab.com".into();
        profile.ssh_alias = "gitlab-work".into();
        service.update_profile(profile).unwrap();
        let repo = service.repository(&rid).unwrap().clone();
        process::git(
            &[
                "remote",
                "set-url",
                "origin",
                "git@gitlab.com:company/team/project.git",
            ],
            Some(Path::new(&repo.path)),
        )
        .unwrap();
        let path = git::config_path(&repo).unwrap();
        let before = fs::read(&path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        assert!(plan.ssh_stanza.contains("HostName gitlab.com"));
        let id = service.apply(&plan.id).unwrap();
        let status = git::inspect(
            &repo,
            Some(service.profile(&pid).unwrap()),
            &service.data.profiles,
        )
        .unwrap();
        assert_eq!(status.remote, "git@gitlab-work:company/team/project.git");
        assert!(status.identity_matches);
        service.validate_ssh(&pid).unwrap();
        assert!(service
            .update_profile(service.profile(&pid).unwrap().clone())
            .is_err());
        service.undo(&id).unwrap();
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
    }
    #[test]
    fn cross_provider_assignment_is_read_only_failure() {
        let (_temp, mut service, pid, rid) = fixture();
        let repo = service.repository(&rid).unwrap().clone();
        process::git(
            &[
                "remote",
                "set-url",
                "origin",
                "git@gitlab.com:company/project.git",
            ],
            Some(Path::new(&repo.path)),
        )
        .unwrap();
        let path = git::config_path(&repo).unwrap();
        let before = fs::read(&path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        assert!(service.plan_assignment(&rid, &pid, true).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
    }
    #[test]
    fn global_defaults_switch_and_restore_without_touching_repositories() {
        let (_temp, mut service, pid, rid) = fixture();
        let original = "# notes\nHost github.com\n    IdentityFile ~/.ssh/original\n    ServerAliveInterval 30\nHost internal\n    HostName internal.example\n";
        fs::write(&service.ssh_config_path, original).unwrap();
        let global_before =
            b"[user]\n name = Original\n email = original@example.com\n[core]\n editor = vim\n";
        fs::write(&service.global_git_path, global_before).unwrap();
        let repo_path = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let repo_before = fs::read(&repo_path).unwrap();
        let first = service.plan_global(&pid).unwrap();
        assert_eq!(fs::read(&service.global_git_path).unwrap(), global_before);
        let tx1 = service.apply(&first.id).unwrap();
        assert_eq!(
            service.data.global_profile_id.as_deref(),
            Some(pid.as_str())
        );
        assert_eq!(global::author(&service.global_git_path).1, "alice@acme.com");
        assert!(fs::read_to_string(&service.global_git_path)
            .unwrap()
            .contains("editor = vim"));
        let mut second = service.profile(&pid).unwrap().clone();
        second.name = "GitLab".into();
        second.provider = provider::GitProvider::Gitlab;
        second.host = "gitlab.com".into();
        second.ssh_alias = "gitlab-work".into();
        second.git_email = "lab@example.com".into();
        service.create_profile(second).unwrap();
        let pid2 = service.data.profiles.last().unwrap().id.clone();
        let plan = service.plan_global(&pid2).unwrap();
        let tx2 = service.apply(&plan.id).unwrap();
        let config = fs::read_to_string(&service.ssh_config_path).unwrap();
        assert!(
            config.ends_with(original),
            "previous provider rules must be restored"
        );
        assert_eq!(fs::read(&repo_path).unwrap(), repo_before);
        service.undo(&tx2).unwrap();
        assert_eq!(
            service.data.global_profile_id.as_deref(),
            Some(pid.as_str())
        );
        service.undo(&tx1).unwrap();
        assert!(service.data.global_profile_id.is_none());
        assert_eq!(fs::read(&service.global_git_path).unwrap(), global_before);
        assert_eq!(
            fs::read_to_string(&service.ssh_config_path).unwrap(),
            original
        );
    }
    #[test]
    fn global_default_preserves_assigned_alias_and_detects_stale_preview() {
        let (_temp, mut service, pid, rid) = fixture();
        let assignment = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&assignment.id).unwrap();
        let before = fs::read(&service.ssh_config_path).unwrap();
        let plan = service.plan_global(&pid).unwrap();
        fs::write(&service.global_git_path, "# external edit\n").unwrap();
        assert!(service.apply(&plan.id).is_err());
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), before);
        let plan = service.plan_global(&pid).unwrap();
        service.apply(&plan.id).unwrap();
        service.validate_ssh(&pid).unwrap();
        assert_eq!(
            service.repository(&rid).unwrap().profile_id.as_deref(),
            Some(pid.as_str())
        );
    }
    #[test]
    fn global_git_include_override_rolls_back_ssh_and_git() {
        let (temp, mut service, pid, _rid) = fixture();
        let include = temp.path().join("override");
        fs::write(&include, "[user]\n email = override@example.com\n").unwrap();
        let original = format!(
            "[user]\n name = Old\n email = old@example.com\n[include]\n path = {}\n",
            include.display()
        );
        fs::write(&service.global_git_path, &original).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let plan = service.plan_global(&pid).unwrap();
        assert!(service.apply(&plan.id).unwrap_err().contains("rolled back"));
        assert_eq!(
            fs::read_to_string(&service.global_git_path).unwrap(),
            original
        );
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
        assert!(service.data.global_profile_id.is_none());
    }
    #[test]
    fn global_new_file_restore_and_external_ssh_conflicts() {
        let (_temp, mut service, pid, _rid) = fixture();
        let plan = service.plan_global(&pid).unwrap();
        let tx = service.apply(&plan.id).unwrap();
        assert!(service
            .update_profile(service.profile(&pid).unwrap().clone())
            .is_err());
        service.undo(&tx).unwrap();
        assert!(!service.global_git_path.exists());
        fs::write(
            &service.ssh_config_path,
            "Host *\n IdentityFile ~/.ssh/extra\n",
        )
        .unwrap();
        assert!(service.plan_global(&pid).is_err());
        assert!(!service.global_git_path.exists());
    }
    #[test]
    fn management_removals_keep_files_and_clear_active_context() {
        let (_temp, mut service, pid, rid) = fixture();
        let repository = service.repository(&rid).unwrap().clone();
        let path = git::config_path(&repository).unwrap();
        let before = fs::read(&path).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let key = service.profile(&pid).unwrap().private_key_path.clone();
        service.remove_repository(&rid).unwrap();
        service.remove_profile(&pid).unwrap();
        assert!(service.data.active_profile_id.is_none());
        assert!(service.data.profiles.is_empty());
        assert!(service.data.repositories.is_empty());
        assert!(Path::new(&key).exists());
        assert!(Path::new(&repository.path).exists());
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
        assert!(service.remove_profile(&pid).is_err());
    }
    #[test]
    fn used_profiles_and_keys_cannot_be_removed_but_can_be_renamed() {
        let (_temp, mut service, pid, rid) = fixture();
        let key = service.profile(&pid).unwrap().public_key_path.clone();
        assert!(service.remove_key_reference(&key).is_err());
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&plan.id).unwrap();
        assert!(service.remove_profile(&pid).is_err());
        let before = fs::read(&service.ssh_config_path).unwrap();
        service.rename_profile(&pid, "Renamed").unwrap();
        assert_eq!(service.profile(&pid).unwrap().name, "Renamed");
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), before);
        service.remove_repository(&rid).unwrap();
        let plan = service.plan_global(&pid).unwrap();
        service.apply(&plan.id).unwrap();
        assert!(service.remove_profile(&pid).is_err());
    }
    #[test]
    fn repository_edit_keeps_assignment_for_same_root_and_clears_for_new_root() {
        let (temp, mut service, pid, rid) = fixture();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&plan.id).unwrap();
        let old = service.repository(&rid).unwrap().clone();
        service
            .update_repository(&rid, "New label", &old.path)
            .unwrap();
        assert_eq!(
            service.repository(&rid).unwrap().profile_id.as_deref(),
            Some(pid.as_str())
        );
        let new_path = temp.path().join("other repo");
        fs::create_dir(&new_path).unwrap();
        process::git(&["init", "--quiet"], Some(&new_path)).unwrap();
        service
            .update_repository(&rid, "Other", new_path.to_str().unwrap())
            .unwrap();
        assert!(service.repository(&rid).unwrap().profile_id.is_none());
        assert!(service.update_repository(&rid, "", &old.path).is_err());
    }
    #[test]
    fn imported_keys_persist_and_removed_references_can_be_reimported() {
        let (_temp, mut service, pid, _rid) = fixture();
        let path = service.profile(&pid).unwrap().public_key_path.clone();
        service.remove_profile(&pid).unwrap();
        service.register_key(&path).unwrap();
        service.register_key(&path).unwrap();
        assert_eq!(
            storage::load(&service.root).unwrap().imported_key_paths,
            vec![path.clone()]
        );
        service.remove_key_reference(&path).unwrap();
        let saved = storage::load(&service.root).unwrap();
        assert!(saved.imported_key_paths.is_empty());
        assert_eq!(saved.hidden_key_paths, vec![path.clone()]);
        assert!(Path::new(&path).exists());
        service.register_key(&path).unwrap();
        assert!(service.data.hidden_key_paths.is_empty());
    }
    #[test]
    fn activation_retargets_alias_without_changing_repositories_then_undoes() {
        let (temp, mut service, pid, rid) = fixture();
        let assignment = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&assignment.id).unwrap();
        let config = git::config_path(service.repository(&rid).unwrap()).unwrap();
        let git_before = fs::read(&config).unwrap();
        let ssh_before = fs::read(&service.ssh_config_path).unwrap();
        let other_key = temp.path().join("second-key");
        process::run(
            "ssh-keygen",
            &[
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-f",
                other_key.to_str().unwrap(),
            ],
            None,
        )
        .unwrap();
        let mut second = service.profile(&pid).unwrap().clone();
        second.name = "Mansi".into();
        second.username = "mansi".into();
        second.git_email = "mansi@example.com".into();
        second.ssh_alias = "github-mansi".into();
        second.public_key_path = format!("{}.pub", other_key.display());
        service.create_profile(second).unwrap();
        let second_id = service.data.profiles.last().unwrap().id.clone();
        let plan = service.plan_activation(&second_id).unwrap();
        assert_eq!(fs::read(&config).unwrap(), git_before);
        let tx = service.apply(&plan.id).unwrap();
        assert!(service.data.single_profile_mode);
        assert_eq!(
            service.data.active_profile_id.as_deref(),
            Some(second_id.as_str())
        );
        assert!(service.repository(&rid).unwrap().profile_id.is_none());
        let alias = service
            .data
            .managed_aliases
            .iter()
            .find(|a| a.id == pid)
            .unwrap();
        assert_eq!(alias.ssh_alias, "github-work");
        assert_eq!(alias.private_key_path, other_key.to_str().unwrap());
        ssh::validate_installed(alias, &service.ssh_config_path).unwrap();
        assert_eq!(
            global::author(&service.global_git_path).1,
            "mansi@example.com"
        );
        assert_eq!(fs::read(&config).unwrap(), git_before);
        let second_plan = service.plan_activation(&pid).unwrap();
        let tx2 = service.apply(&second_plan.id).unwrap();
        assert_eq!(
            service.data.managed_aliases[0].private_key_path,
            service.profile(&pid).unwrap().private_key_path
        );
        service.undo(&tx2).unwrap();
        service.undo(&tx).unwrap();
        assert_eq!(fs::read(&config).unwrap(), git_before);
        assert_eq!(fs::read(&service.ssh_config_path).unwrap(), ssh_before);
        assert!(!service.data.single_profile_mode);
    }
    #[test]
    fn activation_refuses_modified_legacy_alias_without_writes() {
        let (_temp, mut service, pid, rid) = fixture();
        let plan = service.plan_assignment(&rid, &pid, true).unwrap();
        service.apply(&plan.id).unwrap();
        let edited = fs::read_to_string(&service.ssh_config_path)
            .unwrap()
            .replace("IdentitiesOnly yes", "IdentitiesOnly no");
        fs::write(&service.ssh_config_path, &edited).unwrap();
        assert!(service.plan_activation(&pid).is_err());
        assert_eq!(
            fs::read_to_string(&service.ssh_config_path).unwrap(),
            edited
        );
        assert!(!service.global_git_path.exists());
    }
    #[test]
    fn activation_overrides_inherited_global_transport_without_visiting_repositories() {
        let (temp, mut service, pid, rid) = fixture();
        let inherited = temp.path().join("gitshift-active.gitconfig");
        fs::write(&inherited, "[user]\n name = Other\n email = other@example.com\n[core]\n sshCommand = ssh -i old-key\n").unwrap();
        let original = format!("[include]\n path = {}\n", inherited.display());
        fs::write(&service.global_git_path, &original).unwrap();
        // Stale registrations and unrelated local transports cannot block a global switch.
        let repo = service
            .data
            .repositories
            .iter_mut()
            .find(|r| r.id == rid)
            .unwrap();
        repo.profile_id = Some(pid.clone());
        repo.path = temp
            .path()
            .join("missing-repository")
            .to_string_lossy()
            .into_owned();
        let plan = service.plan_activation(&pid).unwrap();
        assert_eq!(service.plans[&plan.id].files.len(), 3);
        let tx = service.apply(&plan.id).unwrap();
        let profile = service.profile(&pid).unwrap();
        global::validate_git(&service.global_git_path, profile).unwrap();
        global::validate_activation_git(
            &service.global_git_path,
            &service.root.join("active-ssh-config"),
        )
        .unwrap();
        assert!(fs::read_to_string(&inherited).unwrap().contains("old-key"));
        let once = fs::read(&service.global_git_path).unwrap();
        let again = service.plan_activation(&pid).unwrap();
        assert_eq!(service.plans[&again.id].files[1].after, once);
        service.undo(&tx).unwrap();
        assert_eq!(
            fs::read_to_string(&service.global_git_path).unwrap(),
            original
        );
    }
    #[test]
    fn activation_isolates_foreign_alias_keys_and_connection_reuse() {
        let (_temp, mut service, pid, _rid) = fixture();
        fs::write(&service.ssh_config_path, "Host github-ep-git\n HostName github.com\n User git\n IdentityFile /tmp/old-ep-key\n CertificateFile /tmp/old-ep-cert.pub\n ControlMaster auto\n ControlPath /tmp/old-ep-socket\n").unwrap();
        let plan = service.plan_activation(&pid).unwrap();
        let tx = service.apply(&plan.id).unwrap();
        let path = service.root.join("active-ssh-config");
        let out = process::run(
            "ssh",
            &["-G", "-F", path.to_str().unwrap(), "github-ep-git"],
            None,
        )
        .unwrap();
        assert_eq!(out.code, 0);
        let identities: Vec<_> = out
            .stdout
            .lines()
            .filter(|l| l.starts_with("identityfile "))
            .collect();
        assert_eq!(
            identities,
            vec![format!(
                "identityfile {}",
                service.profile(&pid).unwrap().private_key_path
            )]
        );
        assert!(out.stdout.lines().any(|l| l == "hostname github.com"));
        assert!(out.stdout.lines().any(|l| l == "controlmaster false"));
        assert!(!out.stdout.contains("old-ep"));
        assert!(out.stdout.lines().any(|l| l == "certificatefile none"));
        assert!(fs::read_to_string(&service.ssh_config_path)
            .unwrap()
            .contains("old-ep-key"));
        service.undo(&tx).unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn health_detects_drift_and_missing_keys_without_executing_ssh_rules() {
        let (_temp, mut service, pid, _) = fixture();
        let plan = service.plan_activation(&pid).unwrap();
        service.apply(&plan.id).unwrap();
        let health = service.health();
        assert!(
            health.checks.iter().all(|c| c.ok),
            "{:?}",
            health.checks.iter().map(|c| &c.detail).collect::<Vec<_>>()
        );
        let transport = service.root.join("active-ssh-config");
        let marker = service.root.join("must-not-exist");
        std::fs::write(
            &transport,
            format!("Match exec \"touch {}\"\n", marker.display()),
        )
        .unwrap();
        assert!(service
            .health()
            .checks
            .iter()
            .any(|c| c.label == "SSH configuration" && !c.ok));
        assert!(!marker.exists());
        std::fs::remove_file(&service.profile(&pid).unwrap().private_key_path).unwrap();
        assert!(service
            .health()
            .checks
            .iter()
            .any(|c| c.label == "SSH key" && !c.ok));
    }

    #[test]
    fn colour_change_does_not_change_active_configuration() {
        let (_temp, mut service, pid, _) = fixture();
        let plan = service.plan_activation(&pid).unwrap();
        service.apply(&plan.id).unwrap();
        let before = std::fs::read(&service.global_git_path).unwrap();
        service.set_color(&pid, "amber").unwrap();
        assert_eq!(service.profile(&pid).unwrap().color, "amber");
        assert_eq!(std::fs::read(&service.global_git_path).unwrap(), before);
        assert!(service.set_color(&pid, "unknown").is_err());
    }
}
