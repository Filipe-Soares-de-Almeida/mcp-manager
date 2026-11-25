import { api, on } from "./api";
import { app } from "./app.svelte";
import { i18n } from "./i18n.svelte";
import type { Release, UpdateCheck } from "./types";

const DAY_MS = 86_400_000;
const DISMISSED_KEY = "mcp-manager.update-dismissed";

/** The update check, the download, and the release notes shown in the Changelog screen. */
class Updates {
  check = $state.raw<UpdateCheck | null>(null);
  checking = $state(false);
  failed = $state(false);
  /** Release notes of the versions up to the one running, newest first. */
  notes = $state.raw<Release[] | null>(null);
  download = $state<{ done: number; total: number | null } | null>(null);
  installing = $state(false);
  installError = $state<string | null>(null);
  dismissed = $state(localStorage.getItem(DISMISSED_KEY));
  #timer: ReturnType<typeof setInterval> | null = null;

  get available(): boolean {
    return !!this.check?.available;
  }

  /** The start card shows until it is dismissed for the latest version. */
  get showCard(): boolean {
    return this.available && this.dismissed !== this.check?.latest;
  }

  async init() {
    await on<{ done: number; total: number | null }>("update-progress", (p) => {
      this.download = p;
      if (p.total !== null && p.done >= p.total) this.installing = true;
    });
    const from = await api.takeUpdatedFrom().catch(() => null);
    if (from) {
      const t = i18n.t.updates;
      app.toast(`${t.updatedTo(from, app.snap?.version ?? "")} ${t.mcpNote}`, {
        action: { label: t.viewChangelog, run: () => (app.page = "changelog") },
        ms: 12000,
      });
    }
    if (app.snap?.startup.check_updates) void this.run(false);
    this.#timer ??= setInterval(() => {
      if (app.snap?.startup.check_updates) void this.run(false);
    }, DAY_MS);
  }

  /** Asks the feed. A check the person started reports a failure; the automatic one stays quiet. */
  async run(manual = true) {
    if (this.checking) return;
    this.checking = true;
    this.failed = false;
    try {
      this.check = await api.checkUpdate(i18n.locale);
    } catch {
      if (manual) this.failed = true;
    } finally {
      this.checking = false;
    }
  }

  async loadNotes() {
    this.notes = await api.releaseNotes(i18n.locale).catch(() => []);
  }

  /** Downloads, verifies and installs the latest version; the app restarts from it. */
  async install() {
    if (this.download || this.installing) return;
    this.installError = null;
    this.download = { done: 0, total: null };
    try {
      // The app normally exits here and restarts from the new version.
      await api.installUpdate();
      this.download = null;
      this.installing = false;
      await app.refresh();
      await this.run(false);
      await this.loadNotes();
    } catch (e) {
      this.download = null;
      this.installing = false;
      this.installError = String(e).replace(/^Error:\s*/, "");
    }
  }

  dismiss() {
    const latest = this.check?.latest;
    if (!latest) return;
    this.dismissed = latest;
    localStorage.setItem(DISMISSED_KEY, latest);
  }
}

export const updates = new Updates();

/** "24 de novembro de 2026" for a `YYYY-MM-DD` day. */
export function releaseDay(day: string | null): string | null {
  const found = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day ?? "");
  if (!found) return null;
  const ms = Date.UTC(Number(found[1]), Number(found[2]) - 1, Number(found[3]));
  return new Intl.DateTimeFormat(i18n.locale, { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" }).format(ms);
}

export function megabytes(bytes: number): string {
  return `${(bytes / 1_048_576).toLocaleString(i18n.locale, { maximumFractionDigits: 1, minimumFractionDigits: 1 })} MB`;
}
