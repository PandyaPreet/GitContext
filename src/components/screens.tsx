import type { ManagementAction } from "./manage-dialog";
import { useState } from "react";
import { MoreHorizontal, Copy, Check, KeyRound, Upload } from "lucide-react";
import type { Workspace } from "../hooks/use-workspace";
import type { Profile } from "../lib/types";
import { api, native } from "../lib/api";
import { duplicateProfile } from "../lib/providers";
import { Button } from "./ui/button";
import { Select } from "./ui/input";
import { Dropdown, MenuItem, MenuSeparator } from "./ui/menu";
import {
  ConnectionState,
  EmptyState,
  IconButton,
  ProviderMark,
} from "./shared";

export function ProfilesScreen({
  workspace: w,
  edit,
  activate,
  manage,
}: {
  workspace: Workspace;
  edit: (p?: Profile) => void;
  activate: (p: Profile) => void;
  manage: (action: ManagementAction) => void;
}) {
  const enabled = native && !w.busy && !w.loading;
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>Profiles</h1>
          <p>Choose one profile. Use it across your Git workflow.</p>
        </div>
        <Button disabled={!enabled} onClick={() => edit()}>
          Add profile
        </Button>
      </div>
      {!w.data.profiles.length ? (
        <EmptyState
          title="No profiles yet"
          description="Create a GitHub or GitLab identity with an existing SSH key."
        />
      ) : (
        <div className="profile-list">
          {w.data.profiles.map((p) => (
            <div
              key={p.id}
              className={`profile-row ${w.data.singleProfileMode && p.id === w.data.globalProfileId ? "active" : ""}`}
            >
              <div
                className={`avatar ${w.data.singleProfileMode && p.id === w.data.globalProfileId ? "active" : ""}`}
              >
                {p.name.slice(0, 1).toUpperCase()}
              </div>
              <div className="profile-technical">
                <h3>{p.name}</h3>
                <div className="subline">
                  <ProviderMark provider={p.provider} />
                  <span>@{p.username}</span>
                </div>
                <div className="mono truncate" title={p.gitEmail}>
                  {p.gitEmail}
                </div>
                <div
                  className="key-line mono truncate"
                  title={p.privateKeyPath}
                >
                  <KeyRound size={12} />
                  {p.privateKeyPath}
                </div>
              </div>
              <div className="profile-connection">
                <ConnectionState
                  value={w.verified[p.id]}
                  busy={w.busy === `Testing ${p.name}`}
                />
                <div className="secondary-line mono truncate" title={p.host}>
                  {p.host}
                </div>
              </div>
              <div className="profile-actions">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={!enabled}
                  onClick={() => void w.testProfile(p)}
                >
                  Test
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={
                    !enabled ||
                    (w.data.singleProfileMode &&
                      p.id === w.data.globalProfileId)
                  }
                  onClick={() => activate(p)}
                >
                  {w.data.singleProfileMode &&
                  p.id === w.data.globalProfileId ? (
                    <>
                      <Check size={13} />
                      Active
                    </>
                  ) : (
                    "Activate"
                  )}
                </Button>
                <Dropdown
                  label={`${p.name} actions`}
                  trigger={
                    <Button
                      aria-label={`${p.name} actions`}
                      title="Profile actions"
                      variant="ghost"
                      size="icon"
                    >
                      <MoreHorizontal size={16} />
                    </Button>
                  }
                >
                  <MenuItem disabled={!enabled} onSelect={() => activate(p)}>
                    Reapply profile…
                  </MenuItem>
                  <MenuItem
                    disabled={
                      !enabled ||
                      p.id === w.data.globalProfileId ||
                      w.data.repositories.some((r) => r.profileId === p.id)
                    }
                    onSelect={() => edit(p)}
                  >
                    Edit identity…
                  </MenuItem>
                  <MenuItem
                    disabled={!enabled}
                    onSelect={() =>
                      manage({ kind: "rename-profile", profile: p })
                    }
                  >
                    Rename profile…
                  </MenuItem>
                  <MenuItem
                    disabled={!enabled}
                    onSelect={() => edit(duplicateProfile(p))}
                  >
                    Duplicate profile…
                  </MenuItem>
                  <MenuSeparator />
                  <MenuItem
                    onSelect={() =>
                      void w.run("Copying SSH alias", () =>
                        navigator.clipboard.writeText(p.sshAlias),
                      )
                    }
                  >
                    Copy SSH alias
                  </MenuItem>
                  <MenuSeparator />
                  <MenuItem
                    disabled={!enabled}
                    onSelect={() =>
                      manage({ kind: "remove-profile", profile: p })
                    }
                  >
                    Remove profile…
                  </MenuItem>
                </Dropdown>
              </div>
            </div>
          ))}
        </div>
      )}
      <p className="context-help">
        Activate sets your Git author and provider SSH key, including old
        aliases managed by Git Context. No repository setup needed.
      </p>
    </>
  );
}

export function KeysScreen({
  workspace: w,
  manage,
}: {
  workspace: Workspace;
  manage: (action: ManagementAction) => void;
}) {
  const enabled = native && !w.busy;
  const importKey = () =>
    w.run("Importing public key", async () => {
      const path = await api.choosePublicKey();
      if (!path) return;
      w.setData(await api.registerKey(path));
      w.setNotice({
        error: false,
        text: "Public key imported. Assign it when creating a profile.",
      });
    });
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>SSH keys</h1>
          <p>Public metadata only. Private keys stay on your computer.</p>
        </div>
        <Button disabled={!enabled} onClick={() => void importKey()}>
          <Upload size={15} />
          Import public key
        </Button>
      </div>
      <div className="key-list">
        {w.keys.map((key) => (
          <div className="key-row" key={key.publicPath}>
            <KeyRound size={16} />
            <div className="grow">
              <div className="key-heading mono truncate" title={key.publicPath}>
                {key.publicPath}
              </div>
              <div className="key-meta">
                <span>{key.keyType}</span>
                <span>{key.inAgent ? "In SSH agent" : "Not in agent"}</span>
                <span>
                  {w.data.profiles
                    .filter((p) => p.publicKeyPath === key.publicPath)
                    .map((p) => p.name)
                    .join(", ") || "Unassigned"}
                </span>
              </div>
              <div
                className="mono secondary-line"
                style={{ overflowWrap: "anywhere" }}
              >
                {key.fingerprint}
              </div>
            </div>
            <IconButton
              label="Copy public key"
              onClick={() =>
                void w.run("Copying public key", async () => {
                  await navigator.clipboard.writeText(key.publicKey);
                  w.setNotice({ error: false, text: "Public key copied." });
                })
              }
            >
              <Copy size={15} />
            </IconButton>
            <Dropdown
              label="Key actions"
              trigger={
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`Actions for ${key.publicPath}`}
                  title="Key actions"
                >
                  <MoreHorizontal size={16} />
                </Button>
              }
            >
              <MenuItem
                disabled={!enabled}
                onSelect={() => manage({ kind: "remove-key", key })}
              >
                Remove key reference…
              </MenuItem>
            </Dropdown>
          </div>
        ))}
      </div>
      {!w.keys.length && (
        <EmptyState
          title="No SSH keys found"
          description="Import an existing .pub file, with its matching private key in the same directory."
        />
      )}
    </>
  );
}

export function SettingsScreen({
  workspace: w,
  restore,
}: {
  workspace: Workspace;
  restore: (id: string) => void;
}) {
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>Settings</h1>
          <p>Appearance, startup, and configuration history.</p>
        </div>
      </div>
      <div className="settings-panel">
        <div className="setting">
          <div>
            <h3>Appearance</h3>
            <p>Choose the application theme.</p>
          </div>
          <Select
            aria-label="Appearance"
            value={w.data.settings.theme}
            disabled={!!w.busy}
            onChange={(e) =>
              void w.updateSettings({
                ...w.data.settings,
                theme: e.target.value as "dark" | "light",
              })
            }
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </Select>
        </div>
        <div className="setting">
          <div>
            <h3>Launch at login</h3>
            <p>Start Git Context when you sign in.</p>
          </div>
          <label className="checkbox-row">
            <input
              type="checkbox"
              aria-label="Launch at login"
              disabled={!native || !!w.busy}
              checked={w.autostart}
              onChange={(e) => void w.setLaunchAtLogin(e.target.checked)}
            />
            <span>{w.autostart ? "On" : "Off"}</span>
          </label>
        </div>
        <div className="setting">
          <div>
            <h3>Startup view</h3>
            <p>Choose where the app opens.</p>
          </div>
          <Select
            aria-label="Startup view"
            disabled={!native || !!w.busy}
            value={
              w.data.settings.startupView === "selector"
                ? "dashboard"
                : w.data.settings.startupView
            }
            onChange={(e) =>
              void w.updateSettings({
                ...w.data.settings,
                startupView: e.target.value as
                  "dashboard" | "tray" | "selector",
              })
            }
          >
            <option value="dashboard">Profiles</option>
            <option value="tray">System tray</option>
          </Select>
        </div>
      </div>
      <section className="section">
        <div className="section-heading">
          <h2>Configuration history</h2>
          <span className="secondary-line">
            Backed up before each assignment
          </span>
        </div>
        {w.transactions.length ? (
          w.transactions.map(([id, state]) => (
            <div className="transaction-row" key={id}>
              <div className="grow">
                <span className="mono">{id}</span>
                <div className="secondary-line">{state}</div>
              </div>
              <Button
                size="sm"
                variant="secondary"
                disabled={
                  !native ||
                  !!w.busy ||
                  !["applied", "pending", "recovery-required"].includes(state)
                }
                onClick={() => restore(id)}
              >
                Restore…
              </Button>
            </div>
          ))
        ) : (
          <p className="muted">No configuration changes recorded.</p>
        )}
        <p className="context-help">
          Restore is refused if the configuration has changed since that
          assignment.
        </p>
      </section>
    </>
  );
}
