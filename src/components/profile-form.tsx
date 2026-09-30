import { useState } from "react";
import { Download, FolderOpen, KeyRound, Loader2 } from "lucide-react";
import { Button } from "./ui/button";
import { Input, Select } from "./ui/input";
import { Field } from "./shared";
import { AccountStatus, KeyGenerator, KeyHandoff } from "./key-setup";
import { useAccountCheck } from "../hooks/use-account-check";
import { api } from "../lib/api";
import { message } from "../lib/utils";
import { newProfile, providers } from "../lib/providers";
import type {
  AppData,
  Detection,
  GitProvider,
  Profile,
  SshKey,
} from "../lib/types";

export function ProfileForm({
  detection,
  availableKeys,
  initial,
  onSave,
  onKeysChanged,
}: {
  detection: Detection | null;
  availableKeys: SshKey[];
  initial?: Profile;
  onSave: (data: AppData, profileId?: string) => void;
  onKeysChanged?: (data: AppData) => void;
}) {
  const [profile, setProfile] = useState<Profile>(initial ?? newProfile());
  const [keys, setKeys] = useState<SshKey[]>(availableKeys);
  const [generating, setGenerating] = useState(
    !initial?.id && availableKeys.length === 0,
  );
  const [freshKey, setFreshKey] = useState("");
  const selectedKey = keys.find((k) => k.publicPath === profile.publicKeyPath);
  const gitAuthor = detection?.gitName || detection?.gitEmail;
  const providerLabel = providers[profile.provider].label;
  const account = useAccountCheck(
    profile.provider,
    profile.host,
    profile.username,
  );
  const generateBlocked = !profile.username.trim()
    ? `Enter your ${providerLabel} username first.`
    : account.checking
      ? `Checking ${providerLabel} account…`
      : account.check?.status === "missing"
        ? "Fix the username before generating a key."
        : undefined;
  const [customHost, setCustomHost] = useState(
    initial?.provider === "gitlab" && initial.host !== "gitlab.com",
  );
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const update = <K extends keyof Profile>(key: K, value: Profile[K]) =>
    setProfile((p) => ({ ...p, [key]: value }));
  function selectKey(key: SshKey) {
    setKeys((old) => [
      ...old.filter((k) => k.publicPath !== key.publicPath),
      key,
    ]);
    setProfile((p) => ({
      ...p,
      publicKeyPath: key.publicPath,
      privateKeyPath: key.privatePath,
    }));
  }
  async function keyGenerated(key: SshKey) {
    selectKey(key);
    setFreshKey(key.publicPath);
    setGenerating(false);
    await api
      .registerKey(key.publicPath)
      .then((data) => onKeysChanged?.(data))
      .catch(() => undefined);
  }
  async function importKey(fromPicker: boolean) {
    setBusy(true);
    setError("");
    try {
      const selected = fromPicker ? await api.choosePublicKey() : path;
      if (selected === null) return;
      selectKey(await api.importKey(selected));
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  function changeProvider(value: string) {
    const provider: GitProvider = value === "github" ? "github" : "gitlab";
    const custom = value === "self-hosted";
    setCustomHost(custom);
    setProfile((p) => ({
      ...p,
      provider,
      host: custom ? "" : providers[provider].host,
      sshPort: 22,
      sshAlias: `${providers[provider].aliasPrefix}${p.sshAlias.replace(/^(github|gitlab)-/, "")}`,
    }));
  }
  return (
    <form
      className="form-stack"
      onSubmit={async (e) => {
        e.preventDefault();
        setBusy(true);
        setError("");
        try {
          const saved = initial?.id
            ? await api.updateProfile(profile)
            : await api.createProfile(profile);
          onSave(saved, initial?.id);
        } catch (e) {
          setError(message(e));
        } finally {
          setBusy(false);
        }
      }}
    >
      <div className="form-grid">
        <Field label="Profile name">
          <Input
            autoFocus
            required
            maxLength={200}
            placeholder="Work"
            value={profile.name}
            onChange={(e) => {
              const name = e.target.value;
              setProfile((p) => ({
                ...p,
                name,
                sshAlias:
                  p.sshAlias ===
                  `${providers[p.provider].aliasPrefix}${p.name.toLowerCase().replace(/[^a-z0-9-]/g, "-")}`
                    ? `${providers[p.provider].aliasPrefix}${name.toLowerCase().replace(/[^a-z0-9-]/g, "-")}`
                    : p.sshAlias,
              }));
            }}
          />
        </Field>
        <Field label="Provider">
          <Select
            value={customHost ? "self-hosted" : profile.provider}
            onChange={(e) => changeProvider(e.target.value)}
          >
            <option value="github">GitHub</option>
            <option value="gitlab">GitLab</option>
            <option value="self-hosted">GitLab · self-hosted</option>
          </Select>
        </Field>
        {customHost && (
          <Field
            label="GitLab hostname"
            hint="Hostname only, without https:// or a path."
          >
            <Input
              required
              placeholder="git.company.com"
              value={profile.host}
              onChange={(e) => update("host", e.target.value)}
            />
          </Field>
        )}
        <div className="field-stack">
          <Field label={`${providers[profile.provider].label} username`}>
            <Input
              required
              placeholder="username"
              maxLength={profile.provider === "github" ? 39 : 255}
              value={profile.username}
              onChange={(e) => update("username", e.target.value)}
              list={profile.provider === "github" ? "github-users" : undefined}
            />
            <datalist id="github-users">
              {detection?.githubUsers.map((u) => (
                <option key={u} value={u} />
              ))}
            </datalist>
          </Field>
          <AccountStatus {...account} providerLabel={providerLabel} />
        </div>
        <Field label="Organization · optional">
          <Input
            maxLength={200}
            placeholder="Company or team"
            value={profile.organization}
            onChange={(e) => update("organization", e.target.value)}
          />
        </Field>
      </div>
      <div className="section-heading import-line">
        <h3>Commit identity</h3>
        {gitAuthor && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() =>
              setProfile((p) => ({
                ...p,
                gitName: detection?.gitName || p.gitName,
                gitEmail: detection?.gitEmail || p.gitEmail,
              }))
            }
          >
            <Download />
            Import Git author
          </Button>
        )}
      </div>
      <div className="form-grid">
        <Field label="Git name">
          <Input
            required
            maxLength={200}
            value={profile.gitName}
            onChange={(e) => update("gitName", e.target.value)}
          />
        </Field>
        <Field
          label="Git email"
          hint={`Add it to your ${providerLabel} account so commits link to you.`}
        >
          <Input
            type="email"
            required
            maxLength={200}
            className="mono"
            value={profile.gitEmail}
            onChange={(e) => update("gitEmail", e.target.value)}
          />
        </Field>
      </div>
      <Field
        label="SSH key"
        hint="Pick an existing key or generate a new one. Private keys never leave this computer."
      >
        <Select
          required
          value={profile.publicKeyPath}
          onChange={(e) => {
            const key = keys.find((k) => k.publicPath === e.target.value);
            if (key) {
              selectKey(key);
              setFreshKey("");
            }
          }}
        >
          <option value="">
            {keys.length ? "Choose an existing key" : "No keys found"}
          </option>
          {keys.map((key) => (
            <option key={key.publicPath} value={key.publicPath}>
              {key.privatePath.split(/[\\/]/).pop()} · {key.keyType}
            </option>
          ))}
          {profile.publicKeyPath &&
            !keys.some((k) => k.publicPath === profile.publicKeyPath) && (
              <option value={profile.publicKeyPath}>
                {profile.publicKeyPath}
              </option>
            )}
        </Select>
      </Field>
      {!generating && (
        <div className="row">
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => importKey(true)}
          >
            <FolderOpen />
            Choose public key
          </Button>
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => setGenerating(true)}
          >
            <KeyRound />
            Generate new key
          </Button>
        </div>
      )}
      {generating && (
        <>
          {!keys.length && (
            <p className="field-hint">
              No SSH keys on this computer yet. Generate one for this profile.
            </p>
          )}
          <KeyGenerator
            profile={profile}
            keys={keys}
            blocked={generateBlocked}
            onGenerated={keyGenerated}
            onCancel={keys.length ? () => setGenerating(false) : undefined}
          />
        </>
      )}
      {selectedKey && !generating && (
        <KeyHandoff
          key={selectedKey.publicPath}
          sshKey={selectedKey}
          profile={profile}
          fresh={freshKey === selectedKey.publicPath}
        />
      )}
      <details className="advanced" open={customHost || undefined}>
        <summary>Advanced settings</summary>
        <div className="form-stack">
          <div className="form-grid">
            <Field label="SSH alias">
              <Input
                required
                value={profile.sshAlias}
                onChange={(e) => update("sshAlias", e.target.value)}
              />
            </Field>
            <Field label="SSH port">
              <Input
                type="number"
                required
                min={1}
                max={65535}
                value={profile.sshPort}
                onChange={(e) => update("sshPort", Number(e.target.value))}
              />
            </Field>
          </div>
          <Field label="Import public key by path">
            <div className="row">
              <Input
                placeholder="~/.ssh/id_ed25519_work.pub"
                value={path}
                onChange={(e) => setPath(e.target.value)}
              />
              <Button
                variant="outline"
                disabled={busy || !path}
                onClick={() => importKey(false)}
              >
                Import
              </Button>
            </div>
          </Field>
        </div>
      </details>
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
      <div className="form-actions">
        <span className="grow muted">Global Git defaults stay unchanged.</span>
        <Button type="submit" disabled={busy || !profile.publicKeyPath}>
          {busy && <Loader2 className="spin" />}
          {initial?.id ? "Save profile" : "Add profile"}
        </Button>
      </div>
    </form>
  );
}
