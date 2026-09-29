import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Command } from "cmdk";
import {
  Users,
  KeyRound,
  Settings,
  GitBranch,
  ChevronDown,
  Search,
  RefreshCw,
  X,
  Loader2,
} from "lucide-react";
import { useWorkspace } from "./hooks/use-workspace";
import { api, native } from "./lib/api";
import type { Plan, Profile } from "./lib/types";
import { providerLabel } from "./lib/providers";
import { Button } from "./components/ui/button";
import { Modal, OperationErrorContext } from "./components/ui/dialog";
import {
  Dropdown,
  MenuItem,
  MenuLabel,
  MenuSeparator,
} from "./components/ui/menu";
import { TooltipProvider } from "./components/ui/tooltip";
import { ConnectionState, IconButton, ProviderMark } from "./components/shared";
import { ProfileForm } from "./components/profile-form";
import {
  KeysScreen,
  ProfilesScreen,
  SettingsScreen,
} from "./components/screens";
import {
  ManageDialog,
  type ManagementAction,
} from "./components/manage-dialog";
const navigation = [
  { id: "profiles", label: "Profiles", icon: Users },
  { id: "keys", label: "SSH keys", icon: KeyRound },
  { id: "settings", label: "Settings", icon: Settings },
] as const;
type Page = (typeof navigation)[number]["id"];
export default function App() {
  const w = useWorkspace();
  const [page, setPage] = useState<Page>("profiles");
  const [editor, setEditor] = useState<{ profile?: Profile } | null>(null);
  const [management, setManagement] = useState<ManagementAction | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [restoreId, setRestoreId] = useState<string | null>(null);
  const [palette, setPalette] = useState(false);
  const enabled = native && !w.loading && !w.busy;
  const active = w.data.singleProfileMode
    ? w.data.profiles.find((p) => p.id === w.data.globalProfileId)
    : undefined;
  const activate = (profile: Profile) =>
    void w.run("Preparing activation", async () => {
      setPlan(await api.planActivation(profile.id));
    });
  const edit = (profile?: Profile) => {
    w.setNotice(null);
    setEditor({ profile });
  };
  const manage = (action: ManagementAction) => {
    w.setNotice(null);
    setManagement(action);
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette((p) => !p);
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  useEffect(() => {
    if (!native) return;
    const subscription = listen<string>("activate-requested", (event) => {
      const profile = w.data.profiles.find((p) => p.id === event.payload);
      if (profile) activate(profile);
    });
    return () => {
      void subscription.then((off) => off());
    };
  }, [w.data.profiles, w.run]);
  return (
    <TooltipProvider delayDuration={350}>
      <OperationErrorContext.Provider
        value={w.notice?.error ? w.notice.text : null}
      >
        <div className="app-shell">
          <aside className="sidebar">
            <div className="brand">
              <GitBranch size={18} />
              <span className="nav-label">Git Context</span>
            </div>
            <nav aria-label="Main navigation">
              {navigation.map((n) => (
                <button
                  key={n.id}
                  className="nav-button"
                  title={n.label}
                  aria-current={page === n.id ? "page" : undefined}
                  onClick={() => setPage(n.id)}
                >
                  <n.icon size={16} />
                  <span className="nav-label">{n.label}</span>
                </button>
              ))}
            </nav>
            <div className="sidebar-bottom sidebar-version nav-label">
              One active profile
            </div>
          </aside>
          <div className="main-shell">
            <header className="toolbar">
              <span className="toolbar-location">
                {navigation.find((n) => n.id === page)?.label}
              </span>
              <div className="spacer" />
              <IconButton
                label="Open command palette"
                onClick={() => setPalette(true)}
              >
                <Search size={16} />
              </IconButton>
              <kbd>⌘ / Ctrl K</kbd>
              <IconButton
                label="Refresh status"
                disabled={!enabled}
                onClick={() => void w.scan()}
              >
                <RefreshCw size={15} />
              </IconButton>
              <Dropdown
                label="Activate profile"
                trigger={
                  <Button
                    variant="ghost"
                    className="profile-trigger"
                    aria-label="Activate profile"
                  >
                    <span className="avatar">
                      {active?.name.slice(0, 1) || "—"}
                    </span>
                    <span className="truncate">
                      {active?.name || "Choose profile"}
                    </span>
                    <ChevronDown size={13} />
                  </Button>
                }
              >
                <MenuLabel>Activate profile</MenuLabel>
                {w.data.profiles.map((p) => (
                  <MenuItem
                    key={p.id}
                    disabled={!enabled}
                    selected={p.id === active?.id}
                    onSelect={() => activate(p)}
                  >
                    <div className="grow">
                      {p.name}
                      <div className="secondary-line">
                        {providerLabel(p.provider)} · {p.gitEmail}
                      </div>
                    </div>
                  </MenuItem>
                ))}
                <MenuSeparator />
                <MenuItem disabled={!enabled} onSelect={() => edit()}>
                  Add profile
                </MenuItem>
              </Dropdown>
            </header>
            <main>
              {!native && (
                <div className="preview-banner">
                  Browser preview · Open the desktop app to access Git and SSH.
                </div>
              )}
              {w.notice && (
                <div
                  className={`notice ${w.notice.error ? "error" : ""}`}
                  role={w.notice.error ? "alert" : "status"}
                >
                  <span>{w.notice.text}</span>
                  <button
                    className="notice-close"
                    aria-label="Dismiss notification"
                    onClick={() => w.setNotice(null)}
                  >
                    <X size={14} />
                  </button>
                </div>
              )}
              {page === "profiles" && (
                <>
                  {!active && !w.loading && (
                    <p className="notice">
                      Activate a profile to set your Git name, email, and
                      provider SSH key. Existing app aliases are included.
                    </p>
                  )}
                  {w.loading ? (
                    <div
                      className="skeleton skeleton-row"
                      aria-label="Loading profiles"
                    />
                  ) : (
                    <ProfilesScreen
                      workspace={w}
                      edit={edit}
                      activate={activate}
                      manage={manage}
                    />
                  )}
                </>
              )}
              {page === "keys" && <KeysScreen workspace={w} manage={manage} />}
              {page === "settings" && (
                <SettingsScreen
                  workspace={w}
                  restore={(id) => {
                    w.setNotice(null);
                    setRestoreId(id);
                  }}
                />
              )}
            </main>
          </div>
          <footer className="statusbar">
            <ProviderMark provider={active?.provider} />
            <span>{active?.name || "No profile activated"}</span>
            <span className="mono truncate" title={active?.gitEmail}>
              {active?.gitEmail}
            </span>
            <span className="mono truncate" title={active?.privateKeyPath}>
              {active?.privateKeyPath.split(/[\\/]/).pop()}
            </span>
            <ConnectionState
              value={active ? w.verified[active.id] : undefined}
            />
            <span className="spacer" />
            {w.busy && (
              <span className="pending-label">
                <Loader2 className="spin" size={12} />
                {w.busy}
              </span>
            )}
          </footer>
        </div>
        {management && (
          <ManageDialog
            action={management}
            workspace={w}
            close={() => setManagement(null)}
          />
        )}
        <Modal
          open={!!editor}
          onOpenChange={(open) => {
            if (!open) setEditor(null);
          }}
          title={editor?.profile?.id ? "Edit profile" : "Add profile"}
          description="Save an account and its SSH key, then activate it."
        >
          {editor && (
            <ProfileForm
              initial={editor.profile}
              availableKeys={w.keys}
              detection={w.detection}
              onSave={(data) => {
                w.setData(data);
                w.setVerified({});
                setEditor(null);
              }}
            />
          )}
        </Modal>
        <Modal
          open={!!plan}
          onOpenChange={(open) => {
            if (!open && !w.busy) setPlan(null);
          }}
          title="Activate profile"
          description="This changes your Git author and provider SSH identity. A backup is saved before applying."
        >
          {plan && (
            <>
              <div className="change-list">
                {plan.changes.map((change) => (
                  <div className="change-row" key={change.label}>
                    <h3>{change.label}</h3>
                    <div className="change-values">
                      <div className="before mono">
                        {change.before || "Not set"}
                      </div>
                      <div className="after mono">{change.after || "None"}</div>
                    </div>
                  </div>
                ))}
              </div>
              <p className="context-help">
                Sets the global Git author and SSH key. Repository-local settings
                can override these defaults. HTTPS credentials are separate.
              </p>
              <div className="form-actions">
                <Button
                  variant="secondary"
                  disabled={!!w.busy}
                  onClick={() => setPlan(null)}
                >
                  Cancel
                </Button>
                <Button
                  disabled={!!w.busy}
                  onClick={() =>
                    void w.run("Activating profile", async () => {
                      await api.apply(plan.id);
                      await w.refresh();
                      setPlan(null);
                      w.setNotice({
                        error: false,
                        text: "Profile activated. Git author and managed SSH identities updated.",
                      });
                    })
                  }
                >
                  {w.busy ? "Activating…" : "Activate profile"}
                </Button>
              </div>
            </>
          )}
        </Modal>
        <Modal
          open={!!restoreId}
          onOpenChange={(open) => {
            if (!open && !w.busy) setRestoreId(null);
          }}
          title="Restore configuration?"
          description="Restore the saved Git and SSH configuration. Newer external changes will not be overwritten."
        >
          <p className="mono">{restoreId}</p>
          <div className="form-actions">
            <Button
              variant="secondary"
              disabled={!!w.busy}
              onClick={() => setRestoreId(null)}
            >
              Cancel
            </Button>
            <Button
              disabled={!!w.busy}
              onClick={() =>
                restoreId &&
                void w.run("Restoring configuration", async () => {
                  await api.undo(restoreId);
                  await w.refresh();
                  setRestoreId(null);
                })
              }
            >
              Restore configuration
            </Button>
          </div>
        </Modal>
        <Modal
          open={palette}
          onOpenChange={setPalette}
          title="Command palette"
          description="Search profiles and actions."
        >
          <Command className="command-palette" loop>
            <Command.Input placeholder="Search profiles and actions…" />
            <Command.List>
              <Command.Empty>No matches.</Command.Empty>
              <Command.Group heading="Activate profile">
                {w.data.profiles.map((p) => (
                  <Command.Item
                    key={p.id}
                    value={`${p.name} ${p.gitEmail} ${providerLabel(p.provider)}`}
                    disabled={!enabled}
                    onSelect={() => {
                      setPalette(false);
                      activate(p);
                    }}
                  >
                    <ProviderMark provider={p.provider} iconOnly />
                    Activate {p.name}
                  </Command.Item>
                ))}
              </Command.Group>
              <Command.Group heading="Actions">
                <Command.Item
                  disabled={!enabled}
                  onSelect={() => {
                    setPalette(false);
                    edit();
                  }}
                >
                  Add profile
                </Command.Item>
                <Command.Item
                  onSelect={() => {
                    setPalette(false);
                    setPage("keys");
                  }}
                >
                  SSH keys
                </Command.Item>
                <Command.Item
                  onSelect={() => {
                    setPalette(false);
                    setPage("settings");
                  }}
                >
                  Settings
                </Command.Item>
              </Command.Group>
            </Command.List>
          </Command>
        </Modal>
      </OperationErrorContext.Provider>
    </TooltipProvider>
  );
}
