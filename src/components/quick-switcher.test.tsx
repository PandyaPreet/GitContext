// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
const mocks = vi.hoisted(() => ({
  activate: vi.fn(),
  dismiss: vi.fn(async () => {}),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("../lib/api", () => ({
  api: {
    snapshot: async () => ({
      profiles: [
        {
          id: "p",
          name: "Personal",
          gitEmail: "me@example.com",
          color: "blue",
        },
      ],
      settings: { theme: "dark" },
    }),
    activateProfile: mocks.activate,
    dismissSwitcher: mocks.dismiss,
  },
}));
import { QuickSwitcher } from "./quick-switcher";
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("keeps the picker open on failure and dismisses only after a successful switch", async () => {
  Element.prototype.scrollIntoView = vi.fn();
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  mocks.activate
    .mockRejectedValueOnce(new Error("Key missing"))
    .mockResolvedValueOnce({});
  const user = userEvent.setup();
  render(<QuickSwitcher />);
  await user.click(await screen.findByText("Personal"));
  await screen.findByRole("alert");
  expect(mocks.dismiss).not.toHaveBeenCalled();
  await user.click(screen.getByText("Personal"));
  await waitFor(() => expect(mocks.dismiss).toHaveBeenCalledTimes(1));
});
