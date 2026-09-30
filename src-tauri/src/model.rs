use crate::provider::{self, GitProvider};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub provider: GitProvider,
    #[serde(default = "provider::default_host")]
    pub host: String,
    #[serde(default = "provider::default_port")]
    pub ssh_port: u16,
    #[serde(alias = "githubUser")]
    pub username: String,
    pub git_name: String,
    pub git_email: String,
    pub private_key_path: String,
    pub public_key_path: String,
    pub ssh_alias: String,
    pub organization: String,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub path: String,
    pub name: String,
    pub profile_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub startup_view: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            startup_view: "dashboard".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub schema_version: u32,
    pub profiles: Vec<Profile>,
    pub repositories: Vec<Repository>,
    pub active_profile_id: Option<String>,
    #[serde(default)]
    pub global_profile_id: Option<String>,
    #[serde(default)]
    pub imported_key_paths: Vec<String>,
    #[serde(default)]
    pub hidden_key_paths: Vec<String>,
    #[serde(default)]
    pub single_profile_mode: bool,
    #[serde(default)]
    pub managed_aliases: Vec<Profile>,
    pub settings: Settings,
}
impl Default for AppData {
    fn default() -> Self {
        Self {
            schema_version: 2,
            profiles: vec![],
            repositories: vec![],
            active_profile_id: None,
            global_profile_id: None,
            imported_key_paths: vec![],
            hidden_key_paths: vec![],
            single_profile_mode: false,
            managed_aliases: vec![],
            settings: Settings::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKey {
    pub private_path: String,
    pub public_path: String,
    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
    pub in_agent: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub git_version: Option<String>,
    pub ssh_version: Option<String>,
    pub git_name: String,
    pub git_email: String,
    pub ssh_config_path: String,
    pub ssh_config_exists: bool,
    pub ssh_hosts: Vec<String>,
    pub ssh_keys: Vec<SshKey>,
    pub agent_available: bool,
    pub gh_available: bool,
    pub github_users: Vec<String>,
    pub platform: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub id: String,
    pub branch: String,
    pub git_name: String,
    pub git_email: String,
    pub remote: String,
    pub dirty: bool,
    pub changes: usize,
    pub remote_info: Option<provider::RemoteInfo>,
    pub ahead_behind: String,
    pub identity_matches: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub label: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub id: String,
    pub repository_id: String,
    pub profile_id: String,
    pub changes: Vec<Change>,
    pub ssh_stanza: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub success: bool,
    pub authenticated_as: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Found,
    Missing,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCheck {
    pub status: AccountStatus,
    pub username: String,
    pub display_name: Option<String>,
    pub profile_url: Option<String>,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, String>;
