<script lang="ts">
  import FirstSteps from "../components/FirstSteps.svelte";
  import Icon from "../components/Icon.svelte";
  import Switch from "../components/Switch.svelte";
  import TextField from "../components/TextField.svelte";
  import { api } from "../api";
  import { app } from "../app.svelte";
  import { confirmer } from "../confirm.svelte";
  import { i18n } from "../i18n.svelte";
  import { kindLabel } from "../labels";
  import { commandLine } from "../spec";
  import type { Row } from "../types";

  let { onedit, onadd }: { onedit: (row: Row) => void; onadd: () => void } = $props();

  const L = $derived(i18n.t);
  const S = $derived(i18n.t.servers);

  let query = $state("");
  let busy = $state<string | null>(null);

  const rows = $derived(app.snap?.rows ?? []);
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((r) => r.name.toLowerCase().includes(q) || commandLine(r.spec).toLowerCase().includes(q));
  });
  const enabled = $derived(filtered.filter((r) => r.enabled));
  const disabled = $derived(filtered.filter((r) => !r.enabled));
  const selfName = $derived(app.snap?.self_entry?.name ?? null);

  async function toggle(row: Row, value: boolean) {
    busy = row.name;
    try {
      const o = await api.setEnabled(row.name, value);
      app.outcome(o, value ? S.enabledDone(row.name) : S.disabledDone(row.name));
    } catch (e) {
      app.fail(e);
    } finally {
      busy = null;
    }
  }

  async function remove(row: Row) {
    const ok = await confirmer.ask({
      title: S.removeTitle(row.name),
      text: row.enabled ? S.removeActiveText : S.removeDisabledText,
      confirm: L.common.remove,
      danger: true,
    });
    if (!ok) return;
    try {
      app.outcome(await api.remove(row.name), S.removedDone(row.name));
    } catch (e) {
      app.fail(e);
    }
  }

  /** Cancels the newest queued change that touches this server. */
  async function undo(row: Row) {
    const names = [row.name, row.renamed_from].filter(Boolean) as string[];
    const last = [...(app.snap?.pending ?? [])].reverse().find((p) => p.names.some((n) => names.includes(n)));
    if (!last) return;
    try {
      await api.cancel(last.id);
      app.toast(S.undone);
      void app.refresh();
    } catch (e) {
      app.fail(e);
    }
  }

  function envCount(row: Row) {
    return Object.keys(row.spec.env ?? {}).length;
  }
</script>

{#snippet item(row: Row)}
  <li class="item" class:off={!row.enabled} class:gone={row.removed}>
    <span class="avatar" aria-hidden="true">{row.name.slice(0, 1).toUpperCase()}</span>
    <div class="text">
      <div class="headline">
        <span class="title-m name">{row.name}</span>
        {#if row.name === selfName}<span class="chip live">{S.thisApp}</span>{/if}
        {#if row.removed}
          <span class="chip queued"><Icon name="schedule" size={16} />{S.willRemove}</span>
        {:else if row.pending.length}
          <span class="chip queued" title={S.queuedTitle}>
            <Icon name="schedule" size={16} />{S.queuedKinds([...new Set(row.pending.map(kindLabel))].join(", "))}
          </span>
        {/if}
        {#if row.renamed_from}<span class="chip">{S.renamedFrom(row.renamed_from)}</span>{/if}
      </div>
      <div class="supporting">
        <span class="code cmd selectable" title={commandLine(row.spec)}>{commandLine(row.spec) || S.noCommand}</span>
        {#if envCount(row)}<span class="body-s variant env">{S.vars(envCount(row))}</span>{/if}
      </div>
    </div>
    <div class="actions">
      {#if row.pending.length || row.removed}
        <button class="icon-btn state" type="button" aria-label={S.undoLast} title={L.common.undo} onclick={() => undo(row)}>
          <Icon name="undo" />
        </button>
      {/if}
      {#if !row.removed}
        <button class="icon-btn state" type="button" aria-label={S.editName(row.name)} title={L.common.edit} onclick={() => onedit(row)}>
          <Icon name="edit" />
        </button>
        <button class="icon-btn state" type="button" aria-label={S.removeName(row.name)} title={L.common.remove} onclick={() => remove(row)}>
          <Icon name="delete" />
        </button>
        <Switch
          checked={row.enabled}
          disabled={busy === row.name}
          label={row.enabled ? S.disableName(row.name) : S.enableName(row.name)}
          onchange={(v) => toggle(row, v)}
        />
      {/if}
    </div>
  </li>
{/snippet}

<div class="page">
  <header class="head">
    <div>
      <h1 class="headline-s">{S.title}</h1>
      <p class="body-m variant">{S.summary(enabled.length, disabled.length)}</p>
    </div>
    <div class="search">
      <TextField label={S.search} bind:value={query}>
        {#snippet leading()}<Icon name="search" />{/snippet}
      </TextField>
    </div>
  </header>

  <FirstSteps />

  {#if app.snap?.config_error}
    <div class="alert error-tone">
      <Icon name="error" />
      <div>
        <p class="title-s">{S.configError}</p>
        <p class="body-m code selectable">{app.snap.config_error}</p>
      </div>
    </div>
  {/if}

  {#if rows.length === 0}
    <div class="empty">
      <Icon name="hub" size={40} />
      <p class="title-m">{S.emptyTitle}</p>
      <p class="body-m variant">{S.emptyText}</p>
      <button class="btn filled with-icon state" type="button" onclick={onadd}><Icon name="add" />{L.nav.addServer}</button>
    </div>
  {:else if filtered.length === 0}
    <div class="empty"><p class="body-m variant">{S.noMatch(query)}</p></div>
  {:else}
    {#if enabled.length}
      <h2 class="title-s section">{S.active}</h2>
      <ul class="list surface-low">
        {#each enabled as row (row.name + row.removed)}{@render item(row)}{/each}
      </ul>
    {/if}
    {#if disabled.length}
      <h2 class="title-s section">{S.disabled}</h2>
      <p class="body-s variant hint">{S.disabledHint}</p>
      <ul class="list surface-low">
        {#each disabled as row (row.name + row.removed)}{@render item(row)}{/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: 8px 24px 96px;
    max-width: 1080px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 8px;
  }
  .head p {
    margin: 4px 0 0;
  }
  .search {
    width: 280px;
  }
  .section {
    margin: 24px 0 8px;
    color: var(--md-on-surface-variant);
  }
  .hint {
    margin: -4px 0 8px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 8px 0;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 72px;
    padding: 8px 16px;
  }
  .item + .item {
    border-top: 1px solid var(--md-outline-variant);
  }
  .avatar {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
    font-weight: 500;
    font-size: 16px;
  }
  .off .avatar {
    background: var(--md-surface-highest);
    color: var(--md-on-surface-variant);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .headline {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .name {
    overflow-wrap: anywhere;
  }
  .off .name {
    color: var(--md-on-surface-variant);
  }
  .gone .name {
    text-decoration: line-through;
    color: var(--md-on-surface-variant);
  }
  .supporting {
    display: flex;
    gap: 12px;
    align-items: baseline;
    margin-top: 2px;
    min-width: 0;
  }
  .cmd {
    color: var(--md-on-surface-variant);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .env {
    flex-shrink: 0;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .actions :global(.switch) {
    margin-left: 12px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 64px 24px;
    color: var(--md-on-surface-variant);
    text-align: center;
  }
  .empty p {
    margin: 0;
  }
  .empty .title-m {
    color: var(--md-on-surface);
  }
  .empty .btn {
    margin-top: 16px;
  }
  .alert {
    display: flex;
    gap: 16px;
    padding: 16px;
    border-radius: var(--shape-m);
    margin: 16px 0;
  }
  .alert p {
    margin: 0;
  }
  .error-tone {
    background: var(--md-error-container);
    color: var(--md-on-error-container);
  }
</style>
