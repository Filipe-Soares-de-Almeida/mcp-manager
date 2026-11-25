import type { ApplyReport, BackupInfo, Outcome, Snapshot, Spec } from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Unlisten = () => void;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  const { mockCall } = await import("./mock");
  return mockCall(cmd, args) as Promise<T>;
}

export async function on<T>(event: string, handler: (payload: T) => void): Promise<Unlisten> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => handler(e.payload));
  }
  const { mockListen } = await import("./mock");
  return mockListen(event, handler as (p: unknown) => void);
}

export const api = {
  state: () => call<Snapshot>("get_state"),
  add: (name: string, spec: Spec) => call<Outcome>("add_server", { name, spec }),
  update: (name: string, newName: string | null, spec: Spec) =>
    call<Outcome>("update_server", { name, newName, spec }),
  remove: (name: string) => call<Outcome>("remove_server", { name }),
  setEnabled: (name: string, enabled: boolean) => call<Outcome>("set_enabled", { name, enabled }),
  cancel: (id: number) => call<number[]>("cancel_change", { id }),
  discard: () => call<number>("discard_changes"),
  applyNow: () => call<ApplyReport>("apply_now"),
  restart: () => call<void>("restart_claude"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  setLocale: (locale: string) => call<void>("set_locale", { locale }),
  setTrayWithClaude: (enabled: boolean) => call<void>("set_tray_with_claude", { enabled }),
  createShortcut: () => call<string>("create_shortcut"),
  backups: () => call<BackupInfo[]>("list_backups"),
  backupServers: (file: string) => call<Record<string, Spec>>("backup_servers", { file }),
  restore: (file: string) => call<Outcome>("restore_backup", { file }),
  backupNow: () => call<string | null>("backup_now"),
  registerSelf: () => call<Outcome>("register_self"),
  selfSpec: () => call<Spec>("self_spec"),
  setConfigOverride: (path: string | null) => call<void>("set_config_override", { path }),
  openPath: (path: string) => call<void>("open_path", { path }),
};
