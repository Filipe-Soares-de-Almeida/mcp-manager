import { api, on } from "./api";
import { i18n, type LocaleSetting } from "./i18n.svelte";
import type { ApplyReport, Outcome, Snapshot } from "./types";

export type Page = "servers" | "changes" | "backups" | "changelog" | "settings";

export interface Toast {
  id: number;
  text: string;
  tone: "info" | "error";
  action?: { label: string; run: () => void };
}

class App {
  snap = $state<Snapshot | null>(null);
  loadError = $state<string | null>(null);
  page = $state<Page>("servers");
  toasts = $state<Toast[]>([]);
  restartStep = $state<string | null>(null);
  #nextToast = 1;

  get restarting() {
    return this.restartStep !== null;
  }

  get restartText() {
    const top = i18n.t.top;
    return this.restartStep ? (top.steps[this.restartStep] ?? top.restarting) : "";
  }

  async refresh() {
    try {
      this.snap = await api.state();
      i18n.set(this.snap.locale.resolved);
      this.loadError = null;
    } catch (e) {
      this.loadError = String(e);
    }
  }

  toast(text: string, opts: { tone?: Toast["tone"]; action?: Toast["action"]; ms?: number } = {}) {
    const id = this.#nextToast++;
    this.toasts = [...this.toasts.slice(-2), { id, text, tone: opts.tone ?? "info", action: opts.action }];
    setTimeout(() => this.dismiss(id), opts.ms ?? (opts.action ? 8000 : 4500));
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  fail(e: unknown) {
    this.toast(String(e).replace(/^Error:\s*/, ""), { tone: "error", ms: 9000 });
  }

  /** Tells the person what became of a change they made in the app. */
  outcome(o: Outcome, done: string) {
    const t = i18n.t.toast;
    if (o.applied) {
      this.toast(o.claude_running ? t.savedNoRestart(done) : t.inConfig(done));
    } else {
      this.toast(t.queued(done), { action: { label: t.restartNow, run: () => this.restart() } });
    }
    void this.refresh();
  }

  async setLocale(setting: LocaleSetting) {
    try {
      await api.setLocale(setting);
      await this.refresh();
    } catch (e) {
      this.fail(e);
    }
  }

  async setAutostart(enabled: boolean) {
    try {
      await api.setAutostart(enabled);
      this.toast(enabled ? i18n.t.toast.autostartOn : i18n.t.toast.autostartOff);
      void this.refresh();
    } catch (e) {
      this.fail(e);
    }
  }

  async setCheckUpdates(enabled: boolean) {
    try {
      await api.setCheckUpdates(enabled);
      void this.refresh();
    } catch (e) {
      this.fail(e);
    }
  }

  async setTrayWithClaude(enabled: boolean) {
    try {
      await api.setTrayWithClaude(enabled);
      void this.refresh();
    } catch (e) {
      this.fail(e);
    }
  }

  async createShortcut() {
    try {
      await api.createShortcut();
      this.toast(i18n.t.toast.shortcutCreated);
      void this.refresh();
    } catch (e) {
      this.fail(e);
    }
  }

  async registerSelf() {
    try {
      this.outcome(await api.registerSelf(), i18n.t.toast.selfRegistered);
    } catch (e) {
      this.fail(e);
    }
  }

  /** The result arrives as "restart-done", also when the tray started it. */
  async restart() {
    if (this.restarting) return;
    this.restartStep = "closing";
    try {
      await api.restart();
    } catch {
      // Reported by "restart-done"; a second start is refused there too.
    }
  }

  reportApplied(r: ApplyReport, restarted: boolean) {
    const t = i18n.t.toast;
    const n = r.applied;
    const what = t.applied(n);
    if (r.failed.length) {
      const details = r.failed.map((f) => `${f.description} (${f.error})`).join("; ");
      this.toast(t.someFailed(what, r.failed.length, details), { tone: "error", ms: 12000 });
    } else if (restarted) {
      this.toast(n ? t.restarted(what) : t.restartedOnly);
    } else if (n) {
      this.toast(t.closedApplied(what));
    }
  }

  async init() {
    await this.refresh();
    await on("state-changed", () => void this.refresh());
    await on<string>("restart-progress", (step) => (this.restartStep = step));
    await on<{ ok: boolean; report: ApplyReport | null; error: string | null }>("restart-done", (d) => {
      this.restartStep = null;
      if (d.ok && d.report) this.reportApplied(d.report, true);
      else this.fail(d.error ?? i18n.t.toast.restartFailed);
      void this.refresh();
    });
    await on<ApplyReport>("applied", (r) => {
      this.reportApplied(r, false);
      void this.refresh();
    });
    await on<string>("apply-error", (e) => this.toast(i18n.t.toast.applyError(e), { tone: "error", ms: 12000 }));
  }
}

export const app = new App();
