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
  mocks.invoke.mockImplementation(
    async (command: string, args: Record<string, unknown>) => {
      switch (command) {
        case "snapshot":
          return structuredClone(initial);
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
it("activates through a reviewed transaction instead of context-only selection", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(
    screen.getAllByRole("button", { name: "Activate" })[0],
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "Activate profile" }),
  );
  expect(mocks.invoke).toHaveBeenCalledWith("plan_activation", {
    profileId: "p1",
  });
  expect(mocks.invoke.mock.calls.some(([c]) => c === "apply_assignment")).toBe(
    false,
  );
  await user.click(dialog.getByRole("button", { name: "Activate profile" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("apply_assignment", {
      planId: "plan1",
    }),
  );
  expect(mocks.invoke.mock.calls.some(([c]) => c === "select_profile")).toBe(
    false,
  );
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
  expect(
    screen.getAllByRole("button", { name: "Active" }),
  ).toHaveLength(1);
  expect(
    screen.getAllByRole("button", { name: "Activate" }),
  ).toHaveLength(1);
});
it("keeps an activation error visible without claiming success", async () => {
  const impl = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation(async (c, a) =>
    c === "apply_assignment"
      ? Promise.reject("Configuration changed since preview")
      : impl(c, a),
  );
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Work" });
  await user.click(
    screen.getAllByRole("button", { name: "Activate" })[0],
  );
  const dialog = within(await screen.findByRole("dialog"));
  await user.click(dialog.getByRole("button", { name: "Activate profile" }));
  expect(await dialog.findByRole("alert")).toHaveTextContent(
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
