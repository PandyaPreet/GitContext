// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { AppData, Detection, Plan } from "./lib/types";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn(async () => false),
}));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: mocks.invoke,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("@tauri-apps/plugin-autostart", () => ({
  enable: mocks.enable,
  disable: mocks.disable,
  isEnabled: mocks.isEnabled,
}));
import App from "./App";

const profile = {
  id: "p1",
  name: "Work",
  username: "alice",
  provider: "github" as const,
  host: "github.com",
  sshPort: 22,
  gitName: "Alice",
  gitEmail: "alice@acme.com",
  privateKeyPath: "/tmp/key",
  publicKeyPath: "/tmp/key.pub",
  sshAlias: "github-work",
  color: "mint" as const,
  organization: "Acme",
};
const initial: AppData = {
  schemaVersion: 2,
  profiles: [
    profile,
    {
      ...profile,
      id: "p2",
      name: "Personal",
      sshAlias: "github-personal",
      color: "violet",
    },
  ],
  repositories: [
    { id: "r1", name: "acme-api", path: "/tmp/acme-api", profileId: null },
  ],
  activeProfileId: "p1",
  settings: { theme: "dark", startupView: "dashboard" },
};
const detection: Detection = {
  gitVersion: "git version 2.50",
  sshVersion: "OpenSSH_9",
  gitName: "Detected Author",
  gitEmail: "detected@example.com",
  sshConfigPath: "/tmp/config",
  sshConfigExists: true,
  sshHosts: [],
  sshKeys: [
    {
      privatePath: "/tmp/key",
      publicPath: "/tmp/key.pub",
      publicKey: "ssh-ed25519 AAAA fixture",
      fingerprint: "SHA256:fixture",
      keyType: "ssh-ed25519",
      inAgent: false,
    },
  ],
  agentAvailable: false,
  ghAvailable: false,
  githubUsers: ["alice"],
  platform: "macos",
  warnings: [],
};
const plan: Plan = {
  id: "plan1",
  repositoryId: "r1",
  profileId: "p1",
  changes: [
    {
      label: "Repository Git email",
      before: "old@example.com",
      after: profile.gitEmail,
    },
  ],
  sshStanza: "Host github-work\n    HostName github.com",
};

beforeEach(() => {
  vi.clearAllMocks();
  Element.prototype.scrollIntoView = vi.fn();
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  let saved = structuredClone(initial);
  mocks.invoke.mockImplementation(
    async (command: string, args: Record<string, unknown>) => {
      switch (command) {
        case "configuration_health":
          return {
            profileId: saved.globalProfileId || null,
            checks: [{ label: "Git author", ok: true, detail: "Matches" }],
          };
        case "shortcut_status":
          return null;
        case "updater_ready":
          return false;
        case "verify_active_identity":
          return {
            success: true,
            authenticatedAs: "alice",
            message: "SSH authentication succeeded",
          };
        case "snapshot":
          return structuredClone(saved);
        case "detect_environment":
          return detection;
        case "list_transactions":
          return [];
        case "repository_status":
          return {
            id: "r1",
            branch: "main",
            gitName: "Previous",
            gitEmail: "old@example.com",
            remote: "https://github.com/acme/api.git",
            dirty: false,
            changes: 0,
            remoteInfo: {
              provider: "github",
              host: "github.com",
              namespace: "acme/api.git",
              organization: "acme",
            },
            aheadBehind: "",
            identityMatches: false,
          };
        case "select_profile":
          return { ...initial, activeProfileId: args.profileId };
        case "plan_activation":
          return plan;
        case "activate_profile":
          saved = {
            ...saved,
            singleProfileMode: true,
            globalProfileId: args.profileId as string,
          };
          return structuredClone(saved);
        case "apply_assignment":
          return "plan1";
        case "create_profile":
          return { ...initial, profiles: [...initial.profiles, args.profile] };
        default:
          throw new Error(`Unexpected command ${command}`);
      }
    },
  );
});
afterEach(cleanup);

it("has no repository module, assignment actions, or background repository scans", async () => {
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  expect(
    screen.queryByRole("button", {
      name: /Repositories|Add repository|Terminal/,
    }),
  ).not.toBeInTheDocument();
  expect(screen.queryByText("acme-api")).not.toBeInTheDocument();
  expect(mocks.invoke.mock.calls.some(([c]) => c === "repository_status")).toBe(
    false,
  );
});
it("activates with one click and no confirmation dialog", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(screen.getAllByRole("button", { name: "Activate" })[0]);
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("activate_profile", {
      profileId: "p1",
    }),
  );
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(mocks.invoke.mock.calls.some(([c]) => c === "select_profile")).toBe(
    false,
  );
});
it("opens read-only configuration details without activating", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(screen.getByRole("button", { name: "View Work details" }));
  const sheet = within(
    await screen.findByRole("dialog", { name: "Work · Configuration" }),
  );
  expect(await sheet.findByText("Repository Git email")).toBeInTheDocument();
  expect(
    mocks.invoke.mock.calls.some(
      ([c]) => c === "activate_profile" || c === "apply_assignment",
    ),
  ).toBe(false);
  expect(
    sheet.queryByRole("button", { name: "Activate profile" }),
  ).not.toBeInTheDocument();
});
it("does not call the old fallback-only global selection active", async () => {
  const impl = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation(async (c, a) =>
    c === "snapshot"
      ? { ...initial, globalProfileId: "p1", singleProfileMode: false }
      : impl(c, a),
  );
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  expect(
    screen.queryByRole("button", { name: "Active" }),
  ).not.toBeInTheDocument();
  expect(screen.getByText("No profile activated")).toBeInTheDocument();
});
it("shows exactly one activated profile", async () => {
  const impl = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation(async (c, a) =>
    c === "snapshot"
      ? { ...initial, globalProfileId: "p2", singleProfileMode: true }
      : impl(c, a),
  );
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  expect(screen.getAllByRole("button", { name: "Active" })).toHaveLength(1);
  expect(screen.getAllByRole("button", { name: "Activate" })).toHaveLength(1);
});
it("keeps an activation error visible without claiming success", async () => {
  const impl = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation(async (c, a) =>
    c === "activate_profile"
      ? Promise.reject("Configuration changed since preview")
      : impl(c, a),
  );
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(screen.getAllByRole("button", { name: "Activate" })[0]);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Configuration changed since preview",
  );
});
it("keeps the palette focused on profile activation", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.keyboard("{Control>}k{/Control}");
  await user.type(
    screen.getByPlaceholderText("Search profiles and actions…"),
    "Personal",
  );
  expect(
    screen.getByRole("option", { name: /Activate Personal/ }),
  ).toBeInTheDocument();
  expect(screen.queryByText("Repositories")).not.toBeInTheDocument();
});
it("preserves profile removal confirmation", async () => {
  const impl = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation(async (c, a) =>
    c === "remove_profile"
      ? { ...initial, profiles: [initial.profiles[1]] }
      : impl(c, a),
  );
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(screen.getByRole("button", { name: "Work actions" }));
  await user.click(screen.getByRole("menuitem", { name: "Remove profile…" }));
  const dialog = within(screen.getByRole("dialog"));
  expect(mocks.invoke.mock.calls.some(([c]) => c === "remove_profile")).toBe(
    false,
  );
  await user.click(dialog.getByRole("button", { name: "Remove from app" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("remove_profile", {
      profileId: "p1",
    }),
  );
});
it("creates provider profiles without implicitly activating them", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(screen.getByRole("button", { name: "Add profile" }));
  const form = within(screen.getByRole("dialog"));
  await user.type(
    form.getByRole("textbox", { name: "Profile name" }),
    "Client",
  );
  await user.selectOptions(
    form.getByRole("combobox", { name: "Provider" }),
    "gitlab",
  );
  await user.type(
    form.getByRole("textbox", { name: "GitLab username" }),
    "alice",
  );
  await user.click(form.getByRole("button", { name: "Import Git author" }));
  await user.selectOptions(
    form.getByRole("combobox", { name: /^SSH key/ }),
    "/tmp/key.pub",
  );
  await user.click(form.getByRole("button", { name: "Add profile" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("create_profile", {
      profile: expect.objectContaining({ provider: "gitlab" }),
    }),
  );
  expect(mocks.invoke.mock.calls.some(([c]) => c === "plan_activation")).toBe(
    false,
  );
});

it("keeps the active identity unchanged until activation succeeds and blocks repeated clicks", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  let rejectActivation!: (reason: string) => void;
  mocks.invoke.mockImplementation(async (command, args) => {
    if (command === "snapshot")
      return { ...initial, singleProfileMode: true, globalProfileId: "p2" };
    if (command === "activate_profile")
      return new Promise((_resolve, reject) => {
        rejectActivation = reject;
      });
    return original(command, args);
  });
  const user = userEvent.setup();
  render(<App />);
  const region = within(
    await screen.findByRole("region", { name: "Current active profile" }),
  );
  await region.findByRole("heading", { name: "Personal" });
  await user.click(screen.getByRole("button", { name: "Activate" }));
  expect(region.getByRole("heading", { name: "Personal" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Activate" })).toBeDisabled();
  expect(
    mocks.invoke.mock.calls.filter(([c]) => c === "activate_profile"),
  ).toHaveLength(1);
  rejectActivation("Could not update configuration");
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Could not update configuration",
  );
  expect(region.getByRole("heading", { name: "Personal" })).toBeInTheDocument();
});

it("keeps active and verified separate until the full identity check succeeds", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Profiles" });
  await user.click(
    screen.getAllByRole("button", { name: "Activate" })[0],
  );
  await screen.findByText(
    "Work is now active. Your Git identity and SSH key are updated.",
  );
  expect(screen.getByText("Identity not verified")).toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Verify active identity" }),
  );
  await screen.findByText(/Verified @alice/);
  expect(mocks.invoke).toHaveBeenCalledWith(
    "verify_active_identity",
    undefined,
  );
});
