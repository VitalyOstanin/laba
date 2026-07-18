import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup, fireEvent, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";

// Mock the Tauri command layer and the external-open helper so the wizard
// exercises its own step/finish logic without a backend.
vi.mock("../api", () => ({
  addServer: vi.fn(async () => {}),
  loginServer: vi.fn(async () => {}),
  ghProbe: vi.fn(async () => "ready"),
}));
vi.mock("../external", () => ({ openExternal: vi.fn(async () => {}) }));

import * as api from "../api";
import SetupWizard from "./SetupWizard.svelte";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const primary = (c: HTMLElement) =>
  c.querySelector<HTMLButtonElement>(".wizard-foot .btn.primary")!;

// Drive the GitHub happy path up to (but not clicking) Finish. GitHub is the
// first backend card; its steps are [backend, connection, verify].
async function driveToVerify(container: HTMLElement): Promise<void> {
  const cards = container.querySelectorAll<HTMLButtonElement>(".wizard-card");
  await fireEvent.click(cards[0]); // GitHub — triggers the ghProbe check
  // Wait for the "gh ready" state so Next is enabled.
  await waitFor(() =>
    expect(container.querySelector(".wizard-ok")).toBeInTheDocument(),
  );
  await fireEvent.click(primary(container)); // -> connection
  const inputs = container.querySelectorAll<HTMLInputElement>(
    ".wizard-field input",
  );
  await fireEvent.input(inputs[0], { target: { value: "gh" } }); // name
  await fireEvent.input(inputs[1], { target: { value: "https://github.com" } }); // url
  await tick();
  await fireEvent.click(primary(container)); // -> verify
  await tick();
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("SetupWizard finish", () => {
  it("creates the GitHub server and both refreshes and closes on success", async () => {
    const onClose = vi.fn();
    const onDone = vi.fn();
    const { container } = render(SetupWizard, { props: { onClose, onDone } });
    await driveToVerify(container);

    await fireEvent.click(primary(container)); // Finish
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));

    expect(api.addServer).toHaveBeenCalledWith(
      "gh",
      "https://github.com",
      "github",
      null,
    );
    // GitHub authenticates via gh, so no token sign-in.
    expect(api.loginServer).not.toHaveBeenCalled();
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("closes the wizard even when the dashboard refresh (onDone) throws", async () => {
    // The profile is created before the refresh, so a failing refresh must not
    // trap the wizard open.
    const onClose = vi.fn();
    const onDone = vi.fn(() => {
      throw new Error("poll restart failed");
    });
    const { container } = render(SetupWizard, { props: { onClose, onDone } });
    await driveToVerify(container);

    await fireEvent.click(primary(container)); // Finish
    await flush();
    await tick();

    expect(api.addServer).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("keeps the wizard open and shows an error when creating the server fails", async () => {
    (api.addServer as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      new Error("kind: boom"),
    );
    const onClose = vi.fn();
    const onDone = vi.fn();
    const { container } = render(SetupWizard, { props: { onClose, onDone } });
    await driveToVerify(container);

    await fireEvent.click(primary(container)); // Finish
    await flush();
    await tick();

    expect(onClose).not.toHaveBeenCalled();
    expect(onDone).not.toHaveBeenCalled();
    expect(container.querySelector(".wizard-error")).toBeInTheDocument();
  });
});
