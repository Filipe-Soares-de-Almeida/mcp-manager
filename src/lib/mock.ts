// In-browser stand-in for the Rust side, used only when the page runs
// outside Tauri (`npm run dev` in a browser) so the UI can be previewed.
import type { BackupInfo, OpKind, Outcome, Pending, Release, Row, Snapshot, Spec } from "./types";

type Listener = (payload: unknown) => void;
const listeners = new Map<string, Set<Listener>>();

function emit(event: string, payload: unknown = null) {
  listeners.get(event)?.forEach((l) => l(payload));
}

export function mockListen(event: string, handler: Listener) {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(handler);
  return () => listeners.get(event)!.delete(handler);
}

const active: Record<string, Spec> = {
  filesystem: { command: "npx", args: ["-y", "@modelcontextprotocol/server-filesystem", "C:\\Users\\me\\Documents"] },
  github: {
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    env: { GITHUB_PERSONAL_ACCESS_TOKEN: "ghp_example_token" },
  },
  "sqlite-tools": { command: "uvx", args: ["mcp-server-sqlite", "--db-path", "C:\\data\\app.db"] },
};
const disabled: Record<string, Spec> = {
  "browser-automation": { command: "node", args: ["C:\\tools\\browser-mcp\\dist\\index.js"], env: { PORT: "9222" } },
};
let pending: (Omit<Pending, "names"> & { name: string; spec?: Spec; newName?: string })[] = [];
let running = true;
const startup = { autostart: false, tray_with_claude: true, shortcut: false, check_updates: true };
let nextId = 1;

// Release notes and an update feed. `?noupdate` (and `?hero`) report the version as the latest;
// `?updated=0.0.9` shows the notice of a start right after an update.
let version = "0.1.0";
const NOTES: Record<string, Release[]> = {
  "en-US": [
    {
      version: "0.3.0", date: "2026-12-10", summary: "Servers can be grouped, and the queue shows what each change will write.",
      groups: [
        { kind: "Added", items: ["Add groups on the Servers screen.", "Add a preview of the config a queued change will write."] },
        { kind: "Fixed", items: ["Fix the editor closing when the list refreshes."] },
      ],
    },
    {
      version: "0.2.0", date: "2026-12-01", summary: null,
      groups: [{ kind: "Changed", items: ["Change the start to be faster when Claude has many servers."] }, { kind: "Fixed", items: ["Fix Restore dropping the disabled servers."] }],
    },
    {
      version: "0.1.0", date: "2026-11-24", summary: "The first release: Claude Desktop's MCP servers in one window, and from Claude's chat.",
      groups: [{ kind: "Added", items: ["Add a window to add, edit, rename, remove, disable and enable Claude Desktop's MCP servers.", "Add a queue that holds changes while Claude runs and writes them once it closes.", "Add in-app updates that download the new version, check its signature and install it."] }],
    },
  ],
  "pt-BR": [
    {
      version: "0.3.0", date: "2026-12-10", summary: "Servidores podem ser agrupados, e a fila mostra o que cada alteração vai gravar.",
      groups: [
        { kind: "Added", items: ["Adiciona grupos na tela de Servidores.", "Adiciona a prévia da config que uma alteração na fila vai gravar."] },
        { kind: "Fixed", items: ["Corrige o editor que fechava quando a lista atualizava."] },
      ],
    },
    {
      version: "0.2.0", date: "2026-12-01", summary: null,
      groups: [{ kind: "Changed", items: ["Altera a abertura para ser mais rápida quando o Claude tem muitos servidores."] }, { kind: "Fixed", items: ["Corrige o Restaurar que perdia os servidores desativados."] }],
    },
    {
      version: "0.1.0", date: "2026-11-24", summary: "A primeira versão: os servidores MCP do Claude Desktop numa janela, e pelo chat do Claude.",
      groups: [{ kind: "Added", items: ["Adiciona uma janela para adicionar, editar, renomear, remover, desativar e ativar os servidores MCP do Claude Desktop.", "Adiciona uma fila que segura as alterações enquanto o Claude roda e as grava quando ele fecha.", "Adiciona a atualização pelo próprio app, que baixa a versão nova, confere a assinatura e a instala."] }],
    },
  ],
};
const newerThan = (lang: string, v: string) =>
  NOTES[lang === "pt-BR" ? "pt-BR" : "en-US"].filter((r) => r.version.localeCompare(v, undefined, { numeric: true }) > 0);

// `?hero` opens the state the README screenshot shows: set up, with the
// app's own entry registered and two changes waiting for Claude to close.
// `?lang=en-US` or `?lang=pt-BR` picks the language.
const params = new URLSearchParams(typeof location !== "undefined" ? location.search : "");
const hero = params.has("hero");
let updatedFrom = params.get("updated");
const latestOffered = params.has("noupdate") || hero ? null : "0.3.0";
let localeSetting = (params.get("lang") ?? "auto") as "auto" | "pt-BR" | "en-US";
const resolveLocale = () =>
  localeSetting !== "auto" ? localeSetting : navigator.language.toLowerCase().startsWith("pt") ? "pt-BR" : "en-US";
const SELF_EXE = "C:\\Apps\\MCP Manager\\mcp-manager.exe";
if (hero) {
  Object.assign(startup, { autostart: true, shortcut: true });
  active["mcp-manager"] = { command: SELF_EXE, args: ["--mcp"] };
  const at = new Date().toISOString();
  pending = [
    {
      id: nextId++, created_at: at, source: "mcp", description: "Add \"brave-search\"", kind: "add", name: "brave-search",
      spec: { command: "npx", args: ["-y", "@modelcontextprotocol/server-brave-search"], env: { BRAVE_API_KEY: "example" } },
    },
    { id: nextId++, created_at: at, source: "app", description: "Disable \"sqlite-tools\"", kind: "disable", name: "sqlite-tools" },
  ];
}

function snapshot(): Snapshot {
  const rows: Row[] = [];
  const touched = new Map<string, OpKind[]>();
  for (const p of pending) touched.set(p.newName ?? p.name, [...(touched.get(p.name) ?? []), p.kind]);
  const sim = { active: { ...active }, disabled: { ...disabled } };
  const removed: Row[] = [];
  for (const p of pending) {
    if (p.kind === "add") sim.active[p.name] = p.spec!;
    if (p.kind === "update" || p.kind === "rename") {
      const where = p.name in sim.active ? sim.active : sim.disabled;
      delete where[p.name];
      where[p.newName ?? p.name] = p.spec!;
    }
    if (p.kind === "disable") {
      sim.disabled[p.name] = sim.active[p.name];
      delete sim.active[p.name];
    }
    if (p.kind === "enable") {
      sim.active[p.name] = sim.disabled[p.name];
      delete sim.disabled[p.name];
    }
    if (p.kind === "remove") {
      const spec = sim.active[p.name] ?? sim.disabled[p.name];
      removed.push({
        name: p.name, spec, enabled: p.name in sim.active, removed: true, pending: ["remove"],
        live: p.name in active ? "active" : p.name in disabled ? "disabled" : null, renamed_from: null,
      });
      delete sim.active[p.name];
      delete sim.disabled[p.name];
    }
  }
  const live = (n: string) => (n in active ? "active" : n in disabled ? "disabled" : null);
  for (const [n, s] of Object.entries(sim.active))
    rows.push({ name: n, spec: s, enabled: true, removed: false, pending: touched.get(n) ?? [], live: live(n), renamed_from: null });
  for (const [n, s] of Object.entries(sim.disabled))
    rows.push({ name: n, spec: s, enabled: false, removed: false, pending: touched.get(n) ?? [], live: live(n), renamed_from: null });
  rows.push(...removed);
  rows.sort((a, b) => a.name.localeCompare(b.name));
  return {
    version,
    locale: { setting: localeSetting, resolved: resolveLocale() },
    startup: { ...startup },
    claude: {
      running,
      processes: running ? 12 : 0,
      install: {
        exe: "C:\\Program Files\\WindowsApps\\Claude_6.28899.6.0_x64__pzs8sxrjxfjjc\\app\\Claude.exe",
        kind: "msix", family: "Claude_pzs8sxrjxfjjc", app_id: "Claude",
        package_dir: "C:\\Program Files\\WindowsApps\\Claude_6.28899.6.0_x64__pzs8sxrjxfjjc",
        version: "6.28899.6.0", found_by: "running",
      },
    },
    config_path: "C:\\Users\\me\\AppData\\Local\\Claude-Data\\claude_pzs8sxrjxfjjc\\claude_desktop_config.json",
    config_source: "detected",
    config_exists: true,
    config_candidates: [
      "C:\\Users\\me\\AppData\\Local\\Claude-Data\\claude_pzs8sxrjxfjjc\\claude_desktop_config.json",
      "C:\\Users\\me\\AppData\\Local\\Packages\\Claude_pzs8sxrjxfjjc\\LocalCache\\Roaming\\Claude\\claude_desktop_config.json",
      "C:\\Users\\me\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
    ],
    config_error: null,
    data_dir: "C:\\Users\\me\\.mcp-manager",
    db_path: "C:\\Users\\me\\.mcp-manager\\mcp-manager.db",
    rows,
    pending: pending.map(({ id, created_at, source, description, kind, name, newName }) => ({
      id, created_at, source, description, kind, names: newName ? [name, newName] : [name],
    })),
    invalid: [],
    history: [
      { id: 2, applied_at: "2026-11-24T18:02:11-03:00", source: "app", description: "Disable \"browser-automation\"", kind: "disable", names: ["browser-automation"], ok: true, message: null, backup: "claude_desktop_config.2026-11-24_18-02-10.512.before-apply.json" },
      { id: 1, applied_at: "2026-11-23T09:40:00-03:00", source: "mcp", description: "Add \"sqlite-tools\"", kind: "add", names: ["sqlite-tools"], ok: true, message: null, backup: null },
    ],
    self_exe: SELF_EXE,
    self_entry: "mcp-manager" in active ? { name: "mcp-manager", up_to_date: true, enabled: true } : null,
  };
}

function queue(kind: OpKind, name: string, extra: { spec?: Spec; newName?: string } = {}): Outcome {
  const descriptions: Record<OpKind, string> = {
    add: `Add "${name}"`, update: `Edit "${name}"`, rename: `Edit "${name}" and rename it to "${extra.newName}"`,
    remove: `Remove "${name}"`, disable: `Disable "${name}"`, enable: `Enable "${name}"`, restore: "Restore",
  };
  const id = nextId++;
  pending.push({ id, created_at: new Date().toISOString(), source: "app", description: descriptions[kind], kind, name, ...extra });
  setTimeout(() => emit("state-changed"), 0);
  return { applied: false, queued_id: id, claude_running: running, message: "queued" };
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function mockCall(cmd: string, args: Record<string, any> = {}): Promise<unknown> {
  await wait(60);
  switch (cmd) {
    case "get_state":
      return snapshot();
    case "add_server":
      return queue("add", args.name, { spec: args.spec });
    case "update_server":
      return queue(args.newName && args.newName !== args.name ? "rename" : "update", args.name, { spec: args.spec, newName: args.newName ?? undefined });
    case "remove_server":
      return queue("remove", args.name);
    case "set_enabled":
      return queue(args.enabled ? "enable" : "disable", args.name);
    case "cancel_change":
      pending = pending.filter((p) => p.id !== args.id);
      emit("state-changed");
      return [];
    case "discard_changes": {
      const n = pending.length;
      pending = [];
      emit("state-changed");
      return n;
    }
    case "restart_claude": {
      for (const step of ["closing", "applying", "starting"]) {
        emit("restart-progress", step);
        await wait(700);
      }
      const n = pending.length;
      pending = [];
      running = true;
      emit("restart-done", { ok: true, report: { applied: n, failed: [], backup: "mock.json" }, error: null });
      emit("state-changed");
      return null;
    }
    case "release_notes":
      return NOTES[args.lang === "pt-BR" ? "pt-BR" : "en-US"].filter(
        (r) => r.version.localeCompare(version, undefined, { numeric: true }) <= 0,
      );
    case "check_update": {
      await wait(500);
      const latest = latestOffered && latestOffered.localeCompare(version, undefined, { numeric: true }) > 0 ? latestOffered : version;
      const pendingVersions = latest === version ? [] : newerThan(args.lang, version);
      return { current: version, latest, available: pendingVersions.length > 0, pending: pendingVersions };
    }
    case "install_update": {
      const total = 4_900_000;
      for (let done = 0; done <= total; done += 700_000) {
        emit("update-progress", { done: Math.min(done, total), total });
        await wait(250);
      }
      emit("update-progress", { done: total, total });
      await wait(900);
      version = latestOffered ?? version;
      emit("state-changed");
      return null;
    }
    case "take_updated_from": {
      const from = updatedFrom;
      updatedFrom = null;
      return from;
    }
    case "set_check_updates":
      startup.check_updates = args.enabled;
      emit("state-changed");
      return null;
    case "set_locale":
      localeSetting = args.locale;
      emit("state-changed");
      return null;
    case "set_autostart":
      startup.autostart = args.enabled;
      emit("state-changed");
      return null;
    case "set_tray_with_claude":
      startup.tray_with_claude = args.enabled;
      emit("state-changed");
      return null;
    case "create_shortcut":
      startup.shortcut = true;
      emit("state-changed");
      return "C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\MCP Manager.lnk";
    case "apply_now":
      pending = [];
      emit("state-changed");
      return { applied: 0, failed: [], backup: null };
    case "list_backups":
      return [
        { file: "claude_desktop_config.2026-11-24_18-02-10.512.before-apply.json", path: "C:\\Users\\me\\.mcp-manager\\backups\\a.json", created: "2026-11-24 18:02:10", reason: "before-apply", size: 12058, servers: 4 },
        { file: "claude_desktop_config.2026-11-24_09-12-44.031.startup.json", path: "C:\\Users\\me\\.mcp-manager\\backups\\b.json", created: "2026-11-24 09:12:44", reason: "startup", size: 11990, servers: 3 },
      ] satisfies BackupInfo[];
    case "backup_servers":
      return { ...active };
    case "restore_backup":
      return queue("restore", "backup");
    case "backup_now":
      return "claude_desktop_config.mock.manual.json";
    case "register_self":
      return queue("add", "mcp-manager", { spec: { command: "C:\\Apps\\MCP Manager\\mcp-manager.exe", args: ["--mcp"] } });
    case "self_spec":
      return { command: "C:\\Apps\\MCP Manager\\mcp-manager.exe", args: ["--mcp"] };
    case "set_config_override":
    case "open_path":
      return null;
  }
  throw new Error(`mock: unknown command ${cmd}`);
}
