export type Spec = Record<string, unknown> & {
  command?: string;
  args?: string[];
  env?: Record<string, string>;
};

export type OpKind = "add" | "update" | "rename" | "remove" | "disable" | "enable" | "restore";

export interface Row {
  name: string;
  spec: Spec;
  enabled: boolean;
  removed: boolean;
  pending: OpKind[];
  live: "active" | "disabled" | null;
  renamed_from: string | null;
}

export interface Pending {
  id: number;
  created_at: string;
  source: string;
  description: string;
  kind: OpKind;
  names: string[];
}

export interface HistoryEntry {
  id: number;
  applied_at: string;
  source: string;
  description: string;
  kind: OpKind | "";
  names: string[];
  ok: boolean;
  message: string | null;
  backup: string | null;
}

export interface Install {
  exe: string;
  kind: "msix" | "squirrel" | "unknown";
  family?: string;
  app_id?: string;
  package_dir?: string;
  version?: string;
  found_by: "running" | "remembered" | "registry" | "squirrel";
}

export interface Snapshot {
  version: string;
  locale: { setting: "auto" | "pt-BR" | "en-US"; resolved: "pt-BR" | "en-US" };
  startup: { autostart: boolean; tray_with_claude: boolean; shortcut: boolean; check_updates: boolean };
  claude: { running: boolean; processes: number; install: Install | null };
  config_path: string;
  config_source: "env" | "override" | "detected";
  config_exists: boolean;
  config_candidates: string[];
  config_error: string | null;
  data_dir: string;
  db_path: string;
  rows: Row[];
  pending: Pending[];
  invalid: { id: number; error: string }[];
  history: HistoryEntry[];
  self_exe: string;
  self_entry: { name: string; up_to_date: boolean; enabled: boolean } | null;
}

export interface NotesGroup {
  kind: string;
  items: string[];
}

export interface Release {
  version: string;
  date: string | null;
  summary: string | null;
  groups: NotesGroup[];
}

export interface UpdateCheck {
  current: string;
  latest: string;
  available: boolean;
  /** The versions newer than the one running, newest first. */
  pending: Release[];
}

export interface Outcome {
  applied: boolean;
  queued_id: number | null;
  claude_running: boolean;
  message: string;
}

export interface ApplyReport {
  applied: number;
  failed: { id: number; description: string; error: string }[];
  backup: string | null;
}

export interface BackupInfo {
  file: string;
  path: string;
  created: string;
  reason: string;
  size: number;
  servers: number | null;
}
