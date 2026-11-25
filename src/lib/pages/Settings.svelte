<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import Switch from "../components/Switch.svelte";
  import TextField from "../components/TextField.svelte";
  import { api } from "../api";
  import { app } from "../app.svelte";
  import { i18n, type LocaleSetting } from "../i18n.svelte";

  const L = $derived(i18n.t);
  const T = $derived(i18n.t.settings);
  const snap = $derived(app.snap);
  const install = $derived(snap?.claude.install ?? null);
  let override = $state("");
  let selfJson = $state("");

  $effect(() => {
    api.selfSpec().then((s) => (selfJson = JSON.stringify({ "mcp-manager": s }, null, 2)), () => {});
  });

  const LANGS: { id: LocaleSetting; label: () => string }[] = [
    { id: "auto", label: () => T.langAuto },
    { id: "pt-BR", label: () => "Português (Brasil)" },
    { id: "en-US", label: () => "English (US)" },
  ];

  async function saveOverride(path: string | null) {
    try {
      await api.setConfigOverride(path);
      override = "";
      app.toast(path ? T.pathSaved : T.pathDetected);
      void app.refresh();
    } catch (e) {
      app.fail(e);
    }
  }

  const register = () => app.registerSelf();

  function folderOf(path: string) {
    return path.replace(/[\\/][^\\/]*$/, "");
  }
</script>

{#if snap}
  <div class="page">
    <h1 class="headline-s">{T.title}</h1>

    <section class="block surface-low">
      <h2 class="title-m">{T.startup}</h2>
      <p class="body-m variant">{T.startupIntro}</p>
      <div class="toggle">
        <div>
          <p class="title-s">{T.autostart}</p>
          <p class="body-s variant">{T.autostartHint}</p>
        </div>
        <Switch checked={snap.startup.autostart} label={T.autostart} onchange={(v) => app.setAutostart(v)} />
      </div>
      <div class="toggle">
        <div>
          <p class="title-s">{T.trayWithClaude}</p>
          <p class="body-s variant">{T.trayWithClaudeHint}</p>
        </div>
        <Switch checked={snap.startup.tray_with_claude} label={T.trayWithClaude} onchange={(v) => app.setTrayWithClaude(v)} />
      </div>
      <div class="toggle">
        <div>
          <p class="title-s">{T.checkUpdates}</p>
          <p class="body-s variant">{T.checkUpdatesHint}</p>
        </div>
        <Switch checked={snap.startup.check_updates} label={T.checkUpdates} onchange={(v) => app.setCheckUpdates(v)} />
      </div>
      <div class="row">
        {#if snap.startup.shortcut}
          <span class="chip live"><Icon name="check" size={16} />{T.shortcutDone}</span>
          <button class="btn text state" type="button" onclick={() => app.createShortcut()}>{T.recreate}</button>
        {:else}
          <button class="btn tonal with-icon state" type="button" onclick={() => app.createShortcut()}>
            <Icon name="add" />{T.createShortcut}
          </button>
        {/if}
      </div>
    </section>

    <section class="block surface-low">
      <h2 class="title-m">{T.language}</h2>
      <div class="segmented" role="radiogroup" aria-label={T.language}>
        {#each LANGS as lang (lang.id)}
          <button
            class="segment state"
            class:selected={snap.locale.setting === lang.id}
            type="button"
            role="radio"
            aria-checked={snap.locale.setting === lang.id}
            onclick={() => app.setLocale(lang.id)}
          >
            {#if snap.locale.setting === lang.id}<Icon name="check" size={18} />{/if}{lang.label()}
          </button>
        {/each}
      </div>
      <p class="body-s variant">{T.languageHint}</p>
    </section>

    <section class="block surface-low">
      <div class="block-head">
        <h2 class="title-m">{T.selfTitle}</h2>
        {#if snap.self_entry?.up_to_date}
          <span class="chip live"><Icon name="check" size={16} />{T.selfRegistered(snap.self_entry.name)}</span>
        {:else if snap.self_entry}
          <span class="chip error">{T.selfOutdated}</span>
        {/if}
      </div>
      <p class="body-m variant">{T.selfIntro} <span class="code">--mcp</span>.</p>
      <pre class="code json selectable">{selfJson}</pre>
      <div class="row">
        {#if !snap.self_entry}
          <button class="btn filled with-icon state" type="button" onclick={register}><Icon name="add" />{T.addToClaude}</button>
        {:else if !snap.self_entry.up_to_date}
          <button class="btn filled with-icon state" type="button" onclick={register}><Icon name="refresh" />{T.updatePath}</button>
        {:else if !snap.self_entry.enabled}
          <p class="body-s variant">{T.selfDisabled}</p>
        {/if}
      </div>
    </section>

    <section class="block surface-low">
      <div class="block-head">
        <h2 class="title-m">Claude Desktop</h2>
        <span class="chip" class:live={snap.claude.running}>
          {snap.claude.running ? T.claudeOpen(snap.claude.processes) : T.claudeClosed}
        </span>
      </div>
      {#if install}
        <dl>
          <dt class="label-m variant">{T.exe}</dt>
          <dd class="code selectable">{install.exe}</dd>
          <dt class="label-m variant">{T.foundBy}</dt>
          <dd class="body-m">{T.found[install.found_by] ?? install.found_by}</dd>
          {#if install.kind === "msix"}
            <dt class="label-m variant">{T.package}</dt>
            <dd class="body-m selectable">{T.packageVersion(install.family ?? "", install.version ?? "")}</dd>
          {/if}
        </dl>
      {:else}
        <p class="body-m variant">{T.notFound}</p>
      {/if}
      <div class="row">
        <button class="btn tonal with-icon state" type="button" disabled={!install || app.restarting} onclick={() => app.restart()}>
          <Icon name={snap.claude.running ? "refresh" : "open"} />{snap.claude.running ? T.restartClaude : T.openClaude}
        </button>
      </div>
    </section>

    <section class="block surface-low">
      <div class="block-head">
        <h2 class="title-m">{T.configTitle}</h2>
        {#if !snap.config_exists}<span class="chip">{T.notYet}</span>{/if}
      </div>
      <p class="code selectable path">{snap.config_path}</p>
      <p class="body-s variant">{T.source[snap.config_source]}</p>
      <details>
        <summary class="body-m">{T.candidates}</summary>
        <ol class="candidates">
          {#each snap.config_candidates as c (c)}
            <li class="code selectable" class:current={c === snap.config_path}>{c}</li>
          {/each}
        </ol>
      </details>
      <div class="override">
        <TextField label={T.otherPath} code bind:value={override} placeholder="C:\…\claude_desktop_config.json" />
        <button class="btn tonal state" type="button" disabled={!override.trim()} onclick={() => saveOverride(override.trim())}>
          {T.use}
        </button>
      </div>
      <div class="row">
        <button class="btn outlined with-icon state" type="button" onclick={() => api.openPath(snap.config_path)}>
          <Icon name="folder" />{L.common.showInFolder}
        </button>
        {#if snap.config_source === "override"}
          <button class="btn text state" type="button" onclick={() => saveOverride(null)}>{T.backToDetected}</button>
        {/if}
      </div>
    </section>

    <section class="block surface-low">
      <h2 class="title-m">{T.dataTitle}</h2>
      <p class="body-m variant">{T.dataIntro}</p>
      <dl>
        <dt class="label-m variant">{T.db}</dt>
        <dd class="code selectable">{snap.db_path}</dd>
        <dt class="label-m variant">{T.folder}</dt>
        <dd class="code selectable">{snap.data_dir}</dd>
      </dl>
      <div class="row">
        <button class="btn outlined with-icon state" type="button" onclick={() => api.openPath(snap.data_dir)}>
          <Icon name="folder" />{L.common.openFolder}
        </button>
      </div>
    </section>

    <p class="body-s variant about">{T.about(snap.version)} <span class="code selectable">{folderOf(snap.self_exe)}</span></p>
  </div>
{/if}

<style>
  .page {
    padding: 8px 24px 48px;
    max-width: 880px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  h1 {
    margin-bottom: 8px;
  }
  .block {
    --field-bg: var(--md-surface-low);
    padding: 20px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .block p {
    margin: 0;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 8px 0;
  }
  .toggle p {
    margin: 0;
  }
  dl {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px 16px;
    margin: 0;
  }
  dt {
    padding-top: 2px;
  }
  dd {
    margin: 0;
    word-break: break-all;
  }
  .path {
    word-break: break-all;
    color: var(--md-on-surface);
  }
  .json {
    margin: 0;
    padding: 12px 16px;
    border-radius: var(--shape-m);
    background: var(--md-surface-lowest);
    white-space: pre-wrap;
    word-break: break-all;
  }
  details summary {
    cursor: pointer;
    color: var(--md-primary);
    width: fit-content;
  }
  .candidates {
    margin: 8px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--md-on-surface-variant);
    word-break: break-all;
  }
  .candidates .current {
    color: var(--md-on-surface);
  }
  .override {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: start;
    margin-top: 4px;
  }
  .override .btn {
    margin-top: 8px;
  }
  .segmented {
    display: inline-flex;
    width: fit-content;
    border: 1px solid var(--md-outline);
    border-radius: 999px;
    overflow: hidden;
  }
  .segment {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 20px;
    border: none;
    background: transparent;
    color: var(--md-on-surface);
    font: inherit;
    font-size: 14px;
    font-weight: 500;
    letter-spacing: 0.1px;
  }
  .segment + .segment {
    border-left: 1px solid var(--md-outline);
  }
  .segment.selected {
    background: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
  }
  .about {
    margin: 8px 0 0;
  }
</style>
