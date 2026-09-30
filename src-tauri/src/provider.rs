use crate::{
    model::{AccountCheck, AccountStatus, Profile, Result, Verification},
    process,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitProvider {
    #[default]
    Github,
    Gitlab,
}

impl GitProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Github => "GitHub",
            Self::Gitlab => "GitLab",
        }
    }
    pub fn default_host(self) -> &'static str {
        match self {
            Self::Github => "github.com",
            Self::Gitlab => "gitlab.com",
        }
    }
    pub fn ssh_keys_url(self, host: &str) -> Result<String> {
        let host = host.to_ascii_lowercase();
        if !valid_host(&host) {
            return Err("Enter a valid hostname for this profile first.".into());
        }
        Ok(match self {
            Self::Github => "https://github.com/settings/ssh/new".into(),
            Self::Gitlab => format!("https://{host}/-/user_settings/ssh_keys"),
        })
    }
    pub fn valid_username(self, username: &str) -> bool {
        !username.is_empty()
            && username.len() <= if self == Self::Github { 39 } else { 255 }
            && username.starts_with(|c: char| {
                c.is_ascii_alphanumeric() || (self == Self::Gitlab && c == '_')
            })
            && username.chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || c == '-'
                    || (self == Self::Gitlab && matches!(c, '.' | '_'))
            })
    }
    fn valid_namespace(self, path: &str) -> bool {
        let segments: Vec<_> = path.split('/').collect();
        (if self == Self::Github {
            segments.len() == 2
        } else {
            (2..=32).contains(&segments.len())
        }) && segments.iter().all(|s| {
            !s.is_empty()
                && !s.starts_with('.')
                && !s.starts_with('-')
                && s.chars().all(|c| {
                    c.is_ascii_alphanumeric()
                        || matches!(c, '.' | '_' | '-')
                        || (self == Self::Gitlab && c == '+')
                })
        })
    }
    pub fn verification(self, text: &str, expected: &str, host: &str) -> Verification {
        let user = text
            .lines()
            .find_map(|line| match self {
                Self::Github => line
                    .strip_prefix("Hi ")
                    .and_then(|v| v.split_once("! You've successfully authenticated"))
                    .map(|(user, _)| user),
                Self::Gitlab => line
                    .strip_prefix("Welcome to GitLab, ")
                    .and_then(|v| v.strip_suffix('!'))
                    .map(|v| v.strip_prefix('@').unwrap_or(v)),
            })
            .filter(|user| self.valid_username(user))
            .map(str::to_string);
        match user {
            Some(user) => {
                let success = user.eq_ignore_ascii_case(expected);
                Verification {
                    success,
                    authenticated_as: Some(user),
                    message: if success {
                        format!("{} confirmed this SSH identity.", self.label())
                    } else {
                        format!("This key belongs to a different {} account. Choose another key or correct the profile username.", self.label())
                    },
                }
            }
            None => {
                let message = if text.contains("Host key verification failed") {
                    format!("Host identity is not trusted. Verify {host}'s SSH host fingerprint with your provider or administrator, then add it to known_hosts outside the app.")
                } else if text.contains("Permission denied") {
                    format!("{} rejected this key. Add its public key to the expected account and load encrypted keys into your SSH agent.", self.label())
                } else {
                    format!("Could not verify SSH at {host}. Check connectivity, the configured SSH port, trusted host keys, and your SSH agent.")
                };
                Verification {
                    success: false,
                    authenticated_as: None,
                    message,
                }
            }
        }
    }
}

/// Looks up a public account by username. Only the username is sent to the provider.
pub fn check_account(provider: GitProvider, host: &str, username: &str) -> Result<AccountCheck> {
    let host = host.to_ascii_lowercase();
    let username = username.trim();
    if !valid_host(&host) || (provider == GitProvider::Github && host != "github.com") {
        return Err("Enter a valid hostname for this profile first.".into());
    }
    if !provider.valid_username(username) {
        return Err(format!(
            "Enter a valid {} username, without the @ prefix.",
            provider.label()
        ));
    }
    let url = match provider {
        GitProvider::Github => format!("https://api.github.com/users/{username}"),
        GitProvider::Gitlab => format!("https://{host}/api/v4/users?username={username}"),
    };
    let output = process::run(
        "curl",
        &[
            "--silent",
            "--proto",
            "=https",
            "--max-time",
            "10",
            "--header",
            "Accept: application/json",
            "--user-agent",
            "GitContext",
            "--write-out",
            "\n%{http_code}",
            &url,
        ],
        None,
    );
    let unknown = |message: String| AccountCheck {
        status: AccountStatus::Unknown,
        username: username.into(),
        display_name: None,
        profile_url: None,
        message,
    };
    let output = match output {
        Ok(output) if output.code == 0 => output,
        _ => {
            return Ok(unknown(format!(
                "Could not reach {}. Check your connection; you can still continue.",
                provider.label()
            )))
        }
    };
    let (body, code) = output
        .stdout
        .rsplit_once('\n')
        .unwrap_or(("", output.stdout.as_str()));
    Ok(parse_account(provider, code.trim(), body, username).unwrap_or_else(unknown))
}

fn parse_account(
    provider: GitProvider,
    code: &str,
    body: &str,
    username: &str,
) -> std::result::Result<AccountCheck, String> {
    let label = provider.label();
    let missing = || AccountCheck {
        status: AccountStatus::Missing,
        username: username.into(),
        display_name: None,
        profile_url: None,
        message: format!("No {label} account named @{username}. Check the spelling."),
    };
    match code {
        "200" => {}
        "404" => return Ok(missing()),
        "401" | "403" | "429" => {
            return Err(format!(
                "{label} did not allow the lookup right now (rate limit or sign-in required). You can still continue."
            ))
        }
        _ => return Err(format!("{label} returned an unexpected response. You can still continue.")),
    }
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| format!("{label} returned an unreadable response."))?;
    let account = match provider {
        GitProvider::Github => {
            if value.get("type").and_then(|v| v.as_str()) == Some("Organization") {
                return Ok(AccountCheck {
                    message: format!(
                        "@{username} is a {label} organization. Use your personal username."
                    ),
                    ..missing()
                });
            }
            value
        }
        GitProvider::Gitlab => match value.as_array().and_then(|users| users.first()) {
            Some(user) => user.clone(),
            None => return Ok(missing()),
        },
    };
    let text = |key: &str| {
        account
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty() && v.len() <= 255 && !v.chars().any(char::is_control))
            .map(str::to_string)
    };
    let canonical = text(if provider == GitProvider::Github {
        "login"
    } else {
        "username"
    })
    .filter(|name| provider.valid_username(name))
    .ok_or_else(|| format!("{label} returned an unreadable response."))?;
    Ok(AccountCheck {
        status: AccountStatus::Found,
        message: format!("{label} account @{canonical} found."),
        username: canonical,
        display_name: text("name"),
        profile_url: text(if provider == GitProvider::Github {
            "html_url"
        } else {
            "web_url"
        })
        .filter(|url| url.starts_with("https://")),
    })
}

pub fn default_host() -> String {
    GitProvider::Github.default_host().into()
}
pub fn default_port() -> u16 {
    22
}
pub fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}
pub fn validate_endpoint(profile: &Profile) -> Result<()> {
    if !valid_host(&profile.host) || profile.ssh_port == 0 {
        return Err(
            "Enter a DNS hostname (without a scheme or path) and an SSH port between 1 and 65535."
                .into(),
        );
    }
    if profile.provider == GitProvider::Github && profile.host != "github.com" {
        return Err("GitHub profiles currently support github.com. Choose GitLab for a self-hosted GitLab instance.".into());
    }
    if profile.provider == GitProvider::Gitlab && profile.host == "github.com" {
        return Err("github.com requires a GitHub profile.".into());
    }
    if !profile.provider.valid_username(&profile.username) {
        return Err(format!(
            "Enter a valid {} username, without the @ prefix.",
            profile.provider.label()
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ParsedRemote {
    pub host: String,
    pub namespace: String,
    pub ssh_port: Option<u16>,
    pub ssh: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub provider: Option<GitProvider>,
    pub host: String,
    pub namespace: String,
    pub organization: String,
}

// Parse only credential-free Git URLs. Do not normalize away traversal or decode
// escapes: both would make a preview differ from the transport Git actually uses.
pub fn parse_remote(raw: &str) -> Result<ParsedRemote> {
    if raw.len() > 2048
        || raw
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '%' | '?' | '#' | '\\'))
    {
        return Err("Unsupported or credential-bearing remote. Review origin in Git before assigning a profile.".into());
    }
    let (authority, path, ssh, url) = if let Some(value) = raw.strip_prefix("https://") {
        let (host, path) = value
            .split_once('/')
            .ok_or("Remote has no repository path")?;
        (host, path, false, true)
    } else if let Some(value) = raw.strip_prefix("ssh://git@") {
        let (host, path) = value
            .split_once('/')
            .ok_or("Remote has no repository path")?;
        (host, path, true, true)
    } else if let Some(value) = raw.strip_prefix("git@") {
        let (host, path) = value.split_once(':').ok_or("Invalid SSH remote")?;
        (host, path, true, false)
    } else {
        return Err(
            "Use a credential-free HTTPS or git@host SSH origin URL before assigning a profile."
                .into(),
        );
    };
    let (host, port) = if url {
        match authority.split_once(':') {
            Some((host, port)) => (
                host,
                Some(
                    port.parse::<u16>()
                        .ok()
                        .filter(|p| *p > 0)
                        .ok_or("Invalid remote port")?,
                ),
            ),
            None => (authority, None),
        }
    } else {
        (authority, None)
    };
    if !valid_host(host) || !GitProvider::Gitlab.valid_namespace(path) {
        return Err("Unsupported remote host or repository namespace. Review origin before assigning a profile.".into());
    }
    Ok(ParsedRemote {
        host: host.to_ascii_lowercase(),
        namespace: path.into(),
        ssh_port: if ssh { port } else { None },
        ssh,
    })
}

pub fn remote_info(raw: &str, profiles: &[Profile]) -> Result<RemoteInfo> {
    let remote = parse_remote(raw)?;
    let alias = if remote.ssh {
        profiles
            .iter()
            .find(|p| p.ssh_alias.eq_ignore_ascii_case(&remote.host))
    } else {
        None
    };
    let host = alias.map(|p| p.host.clone()).unwrap_or(remote.host);
    let provider = match host.as_str() {
        "github.com" => Some(GitProvider::Github),
        "gitlab.com" => Some(GitProvider::Gitlab),
        _ => alias
            .map(|p| p.provider)
            .or_else(|| profiles.iter().find(|p| p.host == host).map(|p| p.provider)),
    };
    if provider.is_some_and(|p| !p.valid_namespace(&remote.namespace)) {
        return Err("Unsupported repository namespace for this provider".into());
    }
    let organization = remote
        .namespace
        .rsplit_once('/')
        .map(|(namespace, _)| namespace.to_string())
        .unwrap_or_default();
    Ok(RemoteInfo {
        provider,
        host,
        namespace: remote.namespace,
        organization,
    })
}

pub fn rewrite_remote(raw: &str, target: &Profile, profiles: &[Profile]) -> Result<String> {
    validate_endpoint(target)?;
    let parsed = parse_remote(raw)?;
    let info = remote_info(raw, profiles)?;
    if info.host != target.host || info.provider != Some(target.provider) {
        return Err(format!("This repository uses {} at {}. Choose a matching profile; assigning {} at {} would change its destination.", info.provider.map(|p| p.label()).unwrap_or("an unconfigured provider"), info.host, target.provider.label(), target.host));
    }
    if parsed.ssh_port.is_some_and(|port| port != target.ssh_port) {
        return Err("The remote SSH port differs from this profile. Use a profile with the repository's SSH port.".into());
    }
    Ok(format!("git@{}:{}", target.ssh_alias, info.namespace))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(provider: GitProvider, host: &str, alias: &str) -> Profile {
        Profile {
            id: "fixture".into(),
            name: "Work".into(),
            provider,
            host: host.into(),
            ssh_port: 22,
            username: "alice".into(),
            git_name: "Alice".into(),
            git_email: "alice@example.com".into(),
            private_key_path: "/tmp/key".into(),
            public_key_path: "/tmp/key.pub".into(),
            ssh_alias: alias.into(),
            organization: String::new(),
            color: "blue".into(),
        }
    }
    #[test]
    fn parses_github_and_gitlab_transports() {
        for (url, provider, path) in [
            (
                "git@github.com:acme/api.git",
                GitProvider::Github,
                "acme/api.git",
            ),
            (
                "https://gitlab.com/acme/team/api.git",
                GitProvider::Gitlab,
                "acme/team/api.git",
            ),
            (
                "ssh://git@gitlab.com/acme/api.git",
                GitProvider::Gitlab,
                "acme/api.git",
            ),
        ] {
            let info = remote_info(url, &[]).unwrap();
            assert_eq!(info.provider, Some(provider));
            assert_eq!(info.namespace, path);
        }
    }
    #[test]
    fn preserves_subgroups_and_resolves_only_registered_aliases() {
        let p = profile(GitProvider::Gitlab, "gitlab.com", "gitlab-company");
        let profiles = vec![p.clone()];
        assert_eq!(
            rewrite_remote("https://gitlab.com/company/team/api.git", &p, &profiles).unwrap(),
            "git@gitlab-company:company/team/api.git"
        );
        assert_eq!(
            remote_info("git@gitlab-company:company/team/api.git", &profiles)
                .unwrap()
                .provider,
            Some(GitProvider::Gitlab)
        );
        assert!(rewrite_remote("git@github-unregistered:acme/api.git", &p, &profiles).is_err());
    }
    #[test]
    fn supports_explicit_self_hosted_gitlab_and_ports() {
        let mut p = profile(GitProvider::Gitlab, "code.acme.test", "gitlab-internal");
        p.ssh_port = 2222;
        assert!(rewrite_remote(
            "ssh://git@code.acme.test:2222/group/team/api.git",
            &p,
            std::slice::from_ref(&p)
        )
        .is_ok());
        assert!(rewrite_remote(
            "ssh://git@code.acme.test:22/group/api.git",
            &p,
            std::slice::from_ref(&p)
        )
        .is_err());
        assert_eq!(
            remote_info("https://unknown.test/team/api", &[])
                .unwrap()
                .provider,
            None
        );
    }
    #[test]
    fn never_moves_a_repository_across_providers_or_hosts() {
        let p = profile(GitProvider::Github, "github.com", "github-personal");
        assert!(rewrite_remote(
            "git@gitlab.com:chotenewsdev/chotenewsadmin.git",
            &p,
            std::slice::from_ref(&p)
        )
        .is_err());
        let p = profile(GitProvider::Gitlab, "code.acme.test", "gitlab-internal");
        assert!(
            rewrite_remote("git@gitlab.com:acme/api.git", &p, std::slice::from_ref(&p)).is_err()
        );
    }
    #[test]
    fn rejects_secrets_injection_traversal_and_unsupported_schemes() {
        for remote in [
            "https://token@github.com/acme/api",
            "https://github.com/a/../b",
            "git@github.com:a/%2e%2e/b",
            "git@gitlab.com:acme/api?token=secret",
            "ssh://git@host:0/acme/api",
            "ssh://git@host:70000/acme/api",
            "file:///tmp/repo",
            "-oProxyCommand=evil",
            "git@gitlab.com:acme/api\nProxyCommand evil",
        ] {
            assert!(parse_remote(remote).is_err(), "{remote}");
        }
    }
    #[test]
    fn parses_provider_greetings_and_account_mismatches() {
        assert!(GitProvider::Github.verification("Hi alice! You've successfully authenticated, but GitHub does not provide shell access.", "alice", "github.com").success);
        assert!(
            GitProvider::Gitlab
                .verification("Welcome to GitLab, @alice.dev!", "alice.dev", "gitlab.com")
                .success
        );
        assert!(
            !GitProvider::Gitlab
                .verification("Welcome to GitLab, @bob!", "alice", "gitlab.com")
                .success
        );
        assert!(
            !GitProvider::Github
                .verification("Welcome to GitLab, @alice!", "alice", "github.com")
                .success
        );
    }
    #[test]
    fn parses_account_lookups() {
        let github = parse_account(
            GitProvider::Github,
            "200",
            r#"{"login":"Alice","name":"Alice A","type":"User","html_url":"https://github.com/Alice"}"#,
            "alice",
        )
        .unwrap();
        assert_eq!(github.status, AccountStatus::Found);
        assert_eq!(github.username, "Alice");
        assert_eq!(github.display_name.as_deref(), Some("Alice A"));
        let org = parse_account(
            GitProvider::Github,
            "200",
            r#"{"login":"acme","type":"Organization"}"#,
            "acme",
        )
        .unwrap();
        assert_eq!(org.status, AccountStatus::Missing);
        let gitlab = parse_account(
            GitProvider::Gitlab,
            "200",
            r#"[{"username":"bob","name":"Bob","web_url":"https://gitlab.com/bob"}]"#,
            "bob",
        )
        .unwrap();
        assert_eq!(
            gitlab.profile_url.as_deref(),
            Some("https://gitlab.com/bob")
        );
        for (provider, code, body) in [
            (GitProvider::Github, "404", "{}"),
            (GitProvider::Gitlab, "200", "[]"),
        ] {
            let result = parse_account(provider, code, body, "nobody").unwrap();
            assert_eq!(result.status, AccountStatus::Missing);
        }
        assert!(parse_account(GitProvider::Github, "403", "", "alice").is_err());
        assert!(parse_account(GitProvider::Github, "200", "not json", "alice").is_err());
    }
    #[test]
    fn ssh_key_settings_urls() {
        assert_eq!(
            GitProvider::Github.ssh_keys_url("github.com").unwrap(),
            "https://github.com/settings/ssh/new"
        );
        assert_eq!(
            GitProvider::Gitlab.ssh_keys_url("Git.Company.com").unwrap(),
            "https://git.company.com/-/user_settings/ssh_keys"
        );
        assert!(GitProvider::Gitlab.ssh_keys_url("evil.com/@x").is_err());
    }
}
