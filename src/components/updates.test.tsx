// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
const mocks = vi.hoisted(() => ({
  ready: vi.fn(),
  check: vi.fn(),
  install: vi.fn(async () => {}),
}));
vi.mock("../lib/api", () => ({
  native: true,
  api: { updaterReady: mocks.ready, shortcutStatus: async () => null },
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));
import { Updates } from "./updates";
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("does not contact the update server when signing is not configured", async () => {
  mocks.ready.mockResolvedValue(false);
  const user = userEvent.setup();
  render(<Updates />);
  await user.click(screen.getByText("Quick switch & app updates"));
  await user.click(screen.getByRole("button", { name: "Check for updates" }));
  await screen.findByText(/configured for signed updates/);
  expect(mocks.check).not.toHaveBeenCalled();
});
it("shows release notes and installs only after the user clicks Install", async () => {
  mocks.ready.mockResolvedValue(true);
  mocks.check.mockResolvedValue({
    version: "0.2.0",
    body: "New profile colours",
    downloadAndInstall: mocks.install,
    close: async () => {},
  });
  const user = userEvent.setup();
  render(<Updates />);
  await user.click(screen.getByText("Quick switch & app updates"));
  await user.click(screen.getByRole("button", { name: "Check for updates" }));
  await screen.findByText("New profile colours");
  expect(mocks.install).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Install update" }));
  await waitFor(() => expect(mocks.install).toHaveBeenCalledTimes(1));
  await screen.findByText(/Update installed/);
});
