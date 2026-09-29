import type { GitProvider, Profile, RepoStatus } from "./types";
export const providers: Record<
  GitProvider,
  { label: string; host: string; aliasPrefix: string }
> = {
  github: { label: "GitHub", host: "github.com", aliasPrefix: "github-" },
  gitlab: { label: "GitLab", host: "gitlab.com", aliasPrefix: "gitlab-" },
};
export const providerLabel = (provider: GitProvider | null | undefined) =>
  provider ? providers[provider].label : "Unknown";
export function compatibleProfile(profile: Profile, status?: RepoStatus) {
  if (!status) return false;
  if (!status.remote) return true;
  const remote = status.remoteInfo;
  return (
    !!remote &&
    remote.host === profile.host &&
    (!remote.provider || remote.provider === profile.provider)
  );
}
export function newProfile(): Profile {
  return {
    id: "",
    name: "",
    provider: "github",
    username: "",
    host: "github.com",
    sshPort: 22,
    gitName: "",
    gitEmail: "",
    privateKeyPath: "",
    publicKeyPath: "",
    sshAlias: "github-",
    organization: "",
    color: "blue",
  };
}
export function duplicateProfile(profile: Profile): Profile {
  return {
    ...profile,
    id: "",
    name: `${profile.name} copy`,
    sshAlias: `${providers[profile.provider].aliasPrefix}${profile.name.toLowerCase().replace(/[^a-z0-9-]/g, "-")}-copy`,
  };
}
