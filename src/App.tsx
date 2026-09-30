import { useEffect, useState } from "react";
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
  CheckCircle2,
} from "lucide-react";
import { IdentityHealth } from "./components/identity-health";
import { Updates } from "./components/updates";
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
  const [detailProfile, setDetailProfile] = useState<Profile | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [restoreId, setRestoreId] = useState<string | null>(null);
  const [palette, setPalette] = useState(false);
  const enabled = native && !w.loading && !w.busy;
  const active = w.data.singleProfileMode
    ? w.data.profiles.find((p) => p.id === w.data.globalProfileId)
    : undefined;
  const activate = (profile: Profile) =>
    void w.run(`Activating ${profile.name}`, async () => {
      const data = await api.activateProfile(profile.id);
      w.setData(data);
      w.setVerified({});
      setDetailProfile(null);
      setPlan(null);
      w.setNotice({
        error: false,
        text: `${profile.name} is now active. Your Git identity and SSH key are updated.`,
      });
      // Keep history current without misreporting a successful switch as failed.
      try {
        await w.refresh();
      } catch {
        /* The returned activation snapshot remains authoritative. */
      }
    });
  const details = (profile: Profile) => {
    setDetailProfile(profile);
    setPlan(null);
    void w.run("Loading profile details", async () => {
      setPlan(await api.planActivation(profile.id));
    });
  };
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
            <div className="sidebar-profile-bottom">
              <Dropdown
                label="Switch profile"
                align="start"
                side="top"
                trigger={
                  <Button
                    variant="ghost"
                    className="profile-trigger sidebar-profile-trigger"
                    aria-label="Switch profile"
                    disabled={!enabled}
                  >
                    <span className={`avatar color-${active?.color || "mint"}`}>
                      {active?.name.slice(0, 1) || "—"}
                    </span>
                    <span className="sidebar-profile-copy">
                      <span className="sidebar-profile-label">
                        Active profile
                      </span>
                      <strong className="truncate">
                        {active?.name || "Choose profile"}
                      </strong>
                      <span className="secondary-line truncate">
                        {active
                          ? providerLabel(active.provider)
                          : "Select an account"}
                      </span>
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
            </header>
            <main>
              <div className="content-container">
                <section
                  className={`active-context color-${active?.color || "mint"} ${active ? "is-active" : ""}`}
                  aria-label="Current active profile"
                >
                  <div className="active-context-mark">
                    {active ? (
                      <CheckCircle2 size={24} />
                    ) : (
                      <GitBranch size={24} />
                    )}
                  </div>
                  <div className="active-context-copy">
                    <span className="active-context-eyebrow">
                      {w.busy.startsWith("Activating")
                        ? w.busy
                        : active
                          ? "Active Git profile"
                          : "Your Git context"}
                    </span>
                    <h2>{active?.name || "Choose a profile to get started"}</h2>
                    <div className="active-context-meta">
                      {active && <ProviderMark provider={active.provider} />}
                      <span className="truncate" title={active?.gitEmail}>
                        {active?.gitEmail ||
                          "One click sets your global Git identity and SSH key."}
                      </span>
                    </div>
                    {active && (
                      <div className="active-context-key">
                        <KeyRound size={13} />
                        <span
                          className="truncate"
                          title={active.privateKeyPath}
                        >
                          {active.privateKeyPath.split(/[\\/]/).pop()}
                        </span>
                        <span>· {active.host}</span>
                      </div>
                    )}
                  </div>
                  {active && (
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={!enabled}
                      onClick={() => details(active)}
                    >
                      View details
                    </Button>
                  )}
                </section>
                <IdentityHealth
                  workspace={w}
                  active={active}
                  activate={activate}
                />
                <Updates />
                {!native && (
                  <div className="preview-banner">
                    Browser preview · Open the desktop app to access Git and
                    SSH.
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
                        details={details}
                        manage={manage}
                      />
                    )}
                  </>
                )}
                {page === "keys" && (
                  <KeysScreen workspace={w} manage={manage} />
                )}
                {page === "settings" && (
                  <SettingsScreen
                    workspace={w}
                    restore={(id) => {
                      w.setNotice(null);
                      setRestoreId(id);
                    }}
                  />
                )}
              </div>
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
          variant="sheet"
          open={!!detailProfile}
          onOpenChange={(open) => {
            if (!open) {
              setDetailProfile(null);
              setPlan(null);
            }
          }}
          title={`${detailProfile?.name || "Profile"} · Configuration`}
          description="See what this profile applies. Activation runs in one click and saves a backup automatically."
        >
          <div className="sheet-summary">
            <ProviderMark provider={detailProfile?.provider} />
            <span>{detailProfile?.host}</span>
            <span className="mono">{detailProfile?.gitEmail}</span>
          </div>
          {!plan && !w.notice?.error && (
            <p className="pending-label">
              <Loader2 size={16} className="spin" /> Loading configuration…
            </p>
          )}
          {plan && (
            <div className="change-list">
              {plan.changes.map((change) => (
                <div className="change-row" key={change.label}>
                  <h3>{change.label}</h3>
                  <div className="change-values">
                    <div>
                      <span className="change-caption">Current</span>
                      <span className="before mono">
                        {change.before || "Not set"}
                      </span>
                    </div>
                    <div>
                      <span className="change-caption">With this profile</span>
                      <span className="after mono">
                        {change.after || "None"}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
          <p className="context-help">
            Global defaults apply to Git author and SSH key. Repository-local
            overrides and HTTPS credentials remain separate. Restore saved
            changes in Settings → Configuration history.
          </p>
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
