export type GitProvider = "github" | "gitlab";
export interface Profile {
  id: string;
  name: string;
  provider: GitProvider;
  username: string;
  host: string;
  sshPort: number;
  gitName: string;
  gitEmail: string;
  privateKeyPath: string;
  publicKeyPath: string;
  sshAlias: string;
  organization: string;
  color: "mint" | "blue" | "violet" | "amber";
}
export interface Repository {
  id: string;
  name: string;
  path: string;
  profileId: string | null;
}
export interface Settings {
  theme: "dark" | "light";
  startupView: "dashboard" | "tray" | "selector";
}
export interface AppData {
  schemaVersion: number;
  profiles: Profile[];
  repositories: Repository[];
  activeProfileId: string | null;
  globalProfileId?: string | null;
  singleProfileMode?: boolean;
  importedKeyPaths?: string[];
  hiddenKeyPaths?: string[];
  settings: Settings;
}
export interface SshKey {
  privatePath: string;
  publicPath: string;
  publicKey: string;
  fingerprint: string;
  keyType: string;
  inAgent: boolean;
}
export interface Detection {
  gitVersion: string | null;
  sshVersion: string | null;
  gitName: string;
  gitEmail: string;
  sshConfigPath: string;
  sshConfigExists: boolean;
  sshHosts: string[];
  sshKeys: SshKey[];
  agentAvailable: boolean;
  ghAvailable: boolean;
  githubUsers: string[];
  platform: string;
  warnings: string[];
}
export interface RepoStatus {
  id: string;
  branch: string;
  gitName: string;
  gitEmail: string;
  remote: string;
  dirty: boolean;
  changes: number;
  remoteInfo: {
    provider: GitProvider | null;
    host: string;
    namespace: string;
    organization: string;
  } | null;
  aheadBehind: string;
  identityMatches: boolean;
}
export interface Plan {
  id: string;
  repositoryId: string;
  profileId: string;
  changes: { label: string; before: string; after: string }[];
  sshStanza: string;
}
export interface Verification {
  success: boolean;
  authenticatedAs: string | null;
  message: string;
}
