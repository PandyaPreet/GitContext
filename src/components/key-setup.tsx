import { useState } from "react";
import {
  AlertTriangle,
  Check,
  CircleCheck,
  CircleX,
  Copy,
  ExternalLink,
  KeyRound,
  Loader2,
} from "lucide-react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Field } from "./shared";
import { api } from "../lib/api";
import { message } from "../lib/utils";
import { providers } from "../lib/providers";
import type { AccountCheck, Profile, SshKey } from "../lib/types";

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function AccountStatus({
  check,
  checking,
  providerLabel,  
}: {
  check: AccountCheck | null;
  checking: boolean;
  providerLabel: string;
}) {
  if (checking)
    return (
      <span className="account-status" role="status">
        <Loader2 size={13} className="spin" />
        Checking {providerLabel}…
      </span>
    );
  if (!check) return null;
  const { status, username, displayName } = check;
  const Icon =
    status === "found"
      ? CircleCheck
      : status === "missing"
        ? CircleX
        : AlertTriangle;
  return (
    <span className={`account-status account-${status}`} role="status">
      <Icon size={13} />
      {status === "found" ? (
        <span className="truncate">
          @{username}
          {displayName && <span className="muted"> · {displayName}</span>}
        </span>
      ) : (
        <span>{check.message}</span>
      )}
    </span>
  );
}

export function suggestKeyName(profile: Profile, keys: SshKey[]) {
  const slug =
    (profile.name || profile.username)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "") || "key";
  const base = `id_ed25519_${profile.provider}_${slug}`.slice(0, 60);
  const taken = new Set(keys.map((k) => fileName(k.privatePath)));
  let name = base;
  for (let n = 2; taken.has(name); n++) name = `${base}_${n}`;
  return name;
}

export function KeyGenerator({
  profile,
  keys,
  blocked,
  onGenerated,
  onCancel,
}: {
  profile: Profile;
  keys: SshKey[];
  blocked?: string;
  onGenerated: (key: SshKey) => Promise<void> | void;
  onCancel?: () => void;
}) {
  const [customName, setName] = useState<string | null>(null);
  const [customComment, setComment] = useState<string | null>(null);
  const name = customName ?? suggestKeyName(profile, keys);
  const comment = customComment ?? profile.gitEmail;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function generate() {
    setBusy(true);
    setError("");
    try {
      await onGenerated(await api.generateKey(name.trim(), comment.trim()));
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="key-panel" aria-label="Generate SSH key">
      <header className="key-panel-header">
        <KeyRound size={16} />
        <div className="grow">
          <h4>Generate a new SSH key</h4>
          <p className="muted">
            Creates an Ed25519 key pair in <code>~/.ssh</code>. The private key
            never leaves this computer.
          </p>
        </div>
      </header>
      <div className="form-grid">
        <Field label="Key file name">
          <Input
            className="mono"
            required
            maxLength={64}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </Field>
        <Field label="Comment · optional">
          <Input
            className="mono"
            maxLength={200}
            placeholder="you@example.com"
            value={comment}
            onChange={(e) => setComment(e.target.value)}
          />
        </Field>
      </div>
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
      <div className="row key-panel-actions">
        {blocked && <span className="grow field-hint">{blocked}</span>}
        {onCancel && (
          <Button variant="ghost" disabled={busy} onClick={onCancel}>
            Cancel
          </Button>
        )}
        <Button
          disabled={busy || !name.trim() || !!blocked}
          onClick={() => void generate()}
        >
          {busy ? <Loader2 className="spin" /> : <KeyRound />}
          Generate key
        </Button>
      </div>
    </section>
  );
}

export function KeyHandoff({
  sshKey,
  profile,
  fresh,
}: {
  sshKey: SshKey;
  profile: Profile;
  fresh: boolean;
}) {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  const provider = providers[profile.provider].label;

  async function copy() {
    setError("");
    try {
      await navigator.clipboard.writeText(sshKey.publicKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setError("Clipboard is unavailable. Select the key above and copy it.");
    }
  }
  async function openSettings() {
    setError("");
    try {
      await api.openKeySettings(profile.provider, profile.host);
    } catch (e) {
      setError(message(e));
    }
  }

  return (
    <section
      className={fresh ? "key-panel key-panel-fresh" : "key-panel"}
      aria-label="Public key"
    >
      <header className="key-panel-header">
        {fresh ? (
          <Check size={16} className="success" />
        ) : (
          <KeyRound size={16} />
        )}
        <div className="grow">
          <h4>
            {fresh
              ? `Key created · add it to ${provider}`
              : fileName(sshKey.privatePath)}
          </h4>
          <p className="muted mono truncate" title={sshKey.fingerprint}>
            {sshKey.keyType} · {sshKey.fingerprint}
          </p>
        </div>
      </header>
      <pre className="key-preview" tabIndex={0} aria-label="Public key text">
        {sshKey.publicKey}
      </pre>
      {fresh ? (
        <ol className="key-steps">
          <li>Copy the public key.</li>
          <li>
            Open {provider} SSH settings
            {profile.username && (
              <>
                {" "}
                signed in as <strong>@{profile.username}</strong>
              </>
            )}
            .
          </li>
          <li>Paste the key, give it a title and save it.</li>
          <li>Save this profile, then run Verify.</li>
        </ol>
      ) : (
        <p className="field-hint">
          Already added to {provider}? You're all set. Otherwise copy it and add
          it in your SSH settings.
        </p>
      )}
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
      <div className="row key-panel-actions">
        <Button variant="outline" onClick={() => void copy()}>
          {copied ? <Check /> : <Copy />}
          {copied ? "Copied" : "Copy public key"}
        </Button>
        <Button
          variant={fresh ? "default" : "outline"}
          disabled={!profile.host}
          onClick={() => void openSettings()}
        >
          <ExternalLink />
          Open {provider} SSH settings
        </Button>
      </div>
    </section>
  );
}
