<script lang="ts">
  import Dialog from "../components/Dialog.svelte";
  import Icon from "../components/Icon.svelte";
  import { api } from "../api";
  import { app } from "../app.svelte";
  import { confirmer } from "../confirm.svelte";
  import { i18n } from "../i18n.svelte";
  import { kilobytes, reasonLabel } from "../labels";
  import { commandLine } from "../spec";
  import type { BackupInfo, Spec } from "../types";

  const L = $derived(i18n.t);
  const B = $derived(i18n.t.backups);

  let list = $state<BackupInfo[]>([]);
  let loading = $state(true);
  let viewing = $state<BackupInfo | null>(null);
  let viewOpen = $state(false);
  let servers = $state<[string, Spec][]>([]);

  async function load() {
    try {
      list = await api.backups();
    } catch (e) {
      app.fail(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Reload whenever the history grows: every apply takes a backup first.
    void app.snap?.history.length;
    void load();
  });

  async function view(b: BackupInfo) {
    try {
      servers = Object.entries(await api.backupServers(b.file));
      viewing = b;
      viewOpen = true;
    } catch (e) {
      app.fail(e);
    }
  }

  async function restore(b: BackupInfo) {
    const ok = await confirmer.ask({
      title: B.restoreTitle,
      text: B.restoreText(b.created, String(b.servers ?? "?")),
      confirm: L.common.restore,
    });
    if (!ok) return;
    try {
      viewOpen = false;
      app.outcome(await api.restore(b.file), B.restored);
    } catch (e) {
      app.fail(e);
    }
  }

  async function backupNow() {
    try {
      const file = await api.backupNow();
      app.toast(file ? B.done : B.noConfig);
      await load();
    } catch (e) {
      app.fail(e);
    }
  }

  const folder = $derived(app.snap ? `${app.snap.data_dir}\\backups` : "");
</script>

<div class="page">
  <header class="head">
    <div>
      <h1 class="headline-s">{B.title}</h1>
      <p class="body-m variant">{B.intro}</p>
    </div>
    <div class="row">
      <button class="btn outlined with-icon state" type="button" onclick={() => api.openPath(folder)}>
        <Icon name="folder" />{L.common.openFolder}
      </button>
      <button class="btn tonal with-icon state" type="button" onclick={backupNow}><Icon name="save" />{B.backupNow}</button>
    </div>
  </header>

  {#if !loading && list.length === 0}
    <p class="empty body-m variant">{B.none}</p>
  {:else}
    <ul class="list surface-low">
      {#each list as b (b.file)}
        <li class="item">
          <span class="lead"><Icon name="history" size={20} /></span>
          <div class="text">
            <p class="body-l">{b.created}</p>
            <p class="body-s variant">
              {reasonLabel(b.reason)} · {B.servers(b.servers)} · {kilobytes(b.size)}
            </p>
          </div>
          <button class="btn text state" type="button" onclick={() => view(b)}>{L.common.view}</button>
          <button class="btn text with-icon state" type="button" onclick={() => restore(b)}>
            <Icon name="restore" />{L.common.restore}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<Dialog bind:open={viewOpen} title={viewing ? B.dialogTitle(viewing.created) : B.dialogFallback} width={640}>
  {#if servers.length === 0}
    <p class="body-m">{B.empty}</p>
  {:else}
    <ul class="mini">
      {#each servers as [name, spec] (name)}
        <li>
          <span class="title-s">{name}</span>
          <span class="code variant selectable">{commandLine(spec)}</span>
        </li>
      {/each}
    </ul>
  {/if}
  {#snippet actions()}
    {#if viewing}
      <button class="btn text state" type="button" onclick={() => viewing && api.openPath(viewing.path)}>{B.showFile}</button>
      <button class="btn text state" type="button" onclick={() => (viewOpen = false)}>{L.common.close}</button>
      <button class="btn filled state" type="button" onclick={() => viewing && restore(viewing)}>{L.common.restore}</button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .page {
    padding: 8px 24px 48px;
    max-width: 1080px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 24px;
  }
  .head p {
    margin: 4px 0 0;
    max-width: 560px;
  }
  .row {
    display: flex;
    gap: 8px;
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
    padding: 8px 12px 8px 16px;
    min-height: 64px;
  }
  .item + .item {
    border-top: 1px solid var(--md-outline-variant);
  }
  .lead {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--md-surface-highest);
    color: var(--md-on-surface-variant);
    flex-shrink: 0;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .text p {
    margin: 0;
  }
  .empty {
    margin: 0;
  }
  .mini {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .mini li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--md-on-surface);
  }
  .mini .code {
    word-break: break-all;
  }
</style>
