/**
 * The `invoke` bridge used by `$lib/api`. In a real Tauri window it forwards to
 * the native bridge; under `vite dev` in a plain browser (no Tauri runtime) it
 * routes to the dev mock so UI work iterates with hot reload and fixtures,
 * without the container build. Detection is by the presence of the Tauri
 * internals global the runtime injects.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { mockInvoke } from "./dev-mock";

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const invoke: <T>(
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<T> = hasTauri
  ? tauriInvoke
  : (cmd, args) => mockInvoke(cmd, args) as Promise<never>;
