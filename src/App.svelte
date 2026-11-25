<script lang="ts">
  import { onMount } from "svelte";
  import Dialog from "./lib/components/Dialog.svelte";
  import Icon, { type IconName } from "./lib/components/Icon.svelte";
  import Logo from "./lib/components/Logo.svelte";
  import ServerDialog from "./lib/components/ServerDialog.svelte";
  import Backups from "./lib/pages/Backups.svelte";
  import Changes from "./lib/pages/Changes.svelte";
  import Servers from "./lib/pages/Servers.svelte";
  import Settings from "./lib/pages/Settings.svelte";
  import { app, type Page } from "./lib/app.svelte";
  import { confirmer } from "./lib/confirm.svelte";
  import { i18n } from "./lib/i18n.svelte";
  import type { Row } from "./lib/types";

  let editorOpen = $state(false);
  let editing = $state<Row | null>(null);

  const L = $derived(i18n.t);
  const NAV: { id: Page; icon: IconName }[] = [
    { id: "servers", icon: "dns" },
    { id: "changes", icon: "schedule" },
    { id: "backups", icon: "history" },
    { id: "settings", icon: "settings" },
  ];

  const snap = $derived(app.snap);
  const pending = $derived(snap?.pending.length ?? 0);
  const running = $derived(snap?.claude.running ?? false);
  const taken = $derived((snap?.rows ?? []).filter((r) => !r.removed).map((r) => r.name));

  function add() {
    editing = null;
    editorOpen = true;
  }

  function edit(row: Row) {
    editing = row;
    editorOpen = true;
  }

  onMount(() => {
    void app.init();
  });
</script>

<div class="shell">
  <nav class="rail" aria-label={L.nav.label}>
    <button class="fab state" type="button" aria-label={L.nav.addServer} title={L.nav.addServer} onclick={add}>
      <Icon name="add" />
    </button>
    <div class="dests">
      {#each NAV as d (d.id)}
        <button
          class="dest"
          class:active={app.page === d.id}
          type="button"
          aria-current={app.page === d.id ? "page" : undefined}
          onclick={() => (app.page = d.id)}
        >
          <span class="indicator state">
            <Icon name={d.icon} />
            {#if d.id === "changes" && pending}<span class="badge">{pending}</span>{/if}
          </span>
          <span class="label-m">{L.nav[d.id]}</span>
        </button>
      {/each}
    </div>
  </nav>

  <div class="main">
    <header class="topbar">
      <span class="brand"><Logo /><span class="title-l">MCP Manager</span></span>
      <div class="status">
        {#if snap}
          <span class="claude body-m" class:on={running}>
            <span class="dot" aria-hidden="true"></span>
            {running ? L.top.claudeOpen : L.top.claudeClosed}
          </span>
        {/if}
        {#if app.restarting}
          <span class="progress body-m"><span class="spinner" aria-hidden="true"></span>{app.restartText}</span>
        {:else if running}
          <button class="btn tonal with-icon state" type="button" onclick={() => app.restart()}>
            <Icon name="refresh" />{L.top.restart}
          </button>
        {/if}
      </div>
    </header>

    {#if pending > 0}
      <div class="banner" role="status">
        <Icon name="schedule" />
        <p class="body-m">
          <strong>{L.banner.count(pending)}</strong>
          {running ? L.banner.running : L.banner.notRunning}
        </p>
        <div class="banner-actions">
          {#if app.page !== "changes"}
            <button class="btn text state" type="button" onclick={() => (app.page = "changes")}>{L.banner.viewQueue}</button>
          {/if}
          {#if running}
            <button class="btn filled state" type="button" disabled={app.restarting} onclick={() => app.restart()}>
              {L.banner.restartNow}
            </button>
          {/if}
        </div>
      </div>
    {/if}

    <main class="content scroll">
      {#if app.loadError && !snap}
        <div class="failed">
          <Icon name="error" size={40} />
          <p class="title-m">{L.loadFailed}</p>
          <p class="body-m code selectable">{app.loadError}</p>
          <button class="btn tonal state" type="button" onclick={() => app.refresh()}>{L.common.retry}</button>
        </div>
      {:else if snap}
        {#if app.page === "servers"}
          <Servers onedit={edit} onadd={add} />
        {:else if app.page === "changes"}
          <Changes />
        {:else if app.page === "backups"}
          <Backups />
        {:else}
          <Settings />
        {/if}
      {/if}
    </main>
  </div>
</div>

<ServerDialog bind:open={editorOpen} row={editing} {taken} />

<Dialog
  bind:open={() => confirmer.open, (v) => !v && confirmer.open && confirmer.answer(false)}
  title={confirmer.opts.title}
  width={440}
>
  {#snippet icon()}<Icon name={confirmer.opts.danger ? "delete" : "restore"} />{/snippet}
  <p class="body-m confirm-text">{confirmer.opts.text}</p>
  {#snippet actions()}
    <button class="btn text state" type="button" onclick={() => confirmer.answer(false)}>{L.common.cancel}</button>
    <button class="btn text state" class:danger={confirmer.opts.danger} type="button" onclick={() => confirmer.answer(true)}>
      {confirmer.opts.confirm}
    </button>
  {/snippet}
</Dialog>

<div class="snackbars" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="snackbar" class:error={t.tone === "error"}>
      <p class="body-m">{t.text}</p>
      {#if t.action}
        <button
          class="btn text state"
          type="button"
          onclick={() => {
            t.action?.run();
            app.dismiss(t.id);
          }}>{t.action.label}</button
        >
      {/if}
      <button class="icon-btn state" type="button" aria-label={L.common.dismissNotice} onclick={() => app.dismiss(t.id)}>
        <Icon name="close" size={20} />
      </button>
    </div>
  {/each}
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
  }
  .rail {
    width: 88px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 40px;
    padding: 20px 0;
    background: var(--md-surface);
  }
  .fab {
    width: 56px;
    height: 56px;
    border-radius: var(--shape-l);
    border: none;
    background: var(--md-cta);
    color: var(--md-on-cta);
    display: grid;
    place-items: center;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3), 0 4px 8px 3px rgb(0 0 0 / 0.15);
  }
  .dests {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .dest {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 80px;
    padding: 0;
    border: none;
    background: none;
    color: var(--md-on-surface-variant);
    font: inherit;
    cursor: pointer;
  }
  .indicator {
    position: relative;
    width: 56px;
    height: 32px;
    border-radius: 16px;
    display: grid;
    place-items: center;
    transition: background 200ms var(--ease);
  }
  .dest:hover .indicator::after {
    opacity: 0.08;
  }
  .active {
    color: var(--md-on-surface);
  }
  .active .indicator {
    background: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
  }
  .badge {
    position: absolute;
    top: -2px;
    left: 32px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--md-primary);
    color: var(--md-on-primary);
    font-size: 11px;
    line-height: 16px;
    font-weight: 500;
    text-align: center;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--md-surface);
  }
  .topbar {
    height: 64px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .claude {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--md-on-surface-variant);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--md-outline);
  }
  .claude.on .dot {
    background: var(--md-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--md-primary) 25%, transparent);
  }
  .progress {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--md-primary);
    height: 40px;
  }
  .spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2.5px solid var(--md-primary);
    border-right-color: transparent;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 16px;
    margin: 0 24px 8px;
    padding: 8px 8px 8px 16px;
    min-height: 56px;
    border-radius: var(--shape-m);
    background: var(--md-tertiary-container);
    color: var(--md-on-tertiary-container);
    max-width: 1032px;
  }
  .banner p {
    flex: 1;
    margin: 0;
  }
  .banner strong {
    font-weight: 500;
  }
  .banner-actions {
    display: flex;
    gap: 8px;
  }
  .banner .btn.text {
    color: var(--md-on-tertiary-container);
  }
  .banner .btn.filled {
    background: var(--md-tertiary);
    color: var(--md-on-tertiary);
  }
  .content {
    flex: 1;
    min-height: 0;
  }
  .failed {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 96px 24px;
    text-align: center;
    color: var(--md-error);
  }
  .failed p {
    margin: 0;
  }
  .failed .code {
    color: var(--md-on-surface-variant);
    max-width: 640px;
  }
  .confirm-text {
    margin: 0;
  }
  .snackbars {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 10;
    width: min(640px, calc(100vw - 48px));
    pointer-events: none;
  }
  .snackbar {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 48px;
    padding: 4px 4px 4px 16px;
    border-radius: var(--shape-xs);
    background: var(--md-inverse-surface);
    color: var(--md-inverse-on-surface);
    box-shadow: 0 2px 6px 2px rgb(0 0 0 / 0.15), 0 1px 2px rgb(0 0 0 / 0.3);
    animation: rise 250ms var(--ease-emph);
  }
  .snackbar p {
    flex: 1;
    margin: 10px 8px 10px 0;
  }
  .snackbar .btn.text {
    color: var(--md-inverse-primary);
  }
  .snackbar .icon-btn {
    color: var(--md-inverse-on-surface);
  }
  .snackbar.error {
    background: var(--md-error-container);
    color: var(--md-on-error-container);
  }
  .snackbar.error .icon-btn {
    color: var(--md-on-error-container);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
