<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { api } from "../api";
  import { app } from "../app.svelte";
  import { confirmer } from "../confirm.svelte";
  import { i18n } from "../i18n.svelte";
  import { describe, sourceLabel, when } from "../labels";

  const L = $derived(i18n.t);
  const C = $derived(i18n.t.changes);
  const pending = $derived(app.snap?.pending ?? []);
  const history = $derived(app.snap?.history ?? []);
  const invalid = $derived(new Map((app.snap?.invalid ?? []).map((i) => [i.id, i.error])));
  const running = $derived(app.snap?.claude.running ?? false);

  async function cancel(id: number) {
    try {
      const dropped = await api.cancel(id);
      app.toast(dropped.length ? C.cancelledWith(dropped.length) : C.cancelled);
      void app.refresh();
    } catch (e) {
      app.fail(e);
    }
  }

  async function discard() {
    const ok = await confirmer.ask({
      title: C.discardTitle,
      text: C.discardText(pending.length),
      confirm: C.discard,
      danger: true,
    });
    if (!ok) return;
    try {
      await api.discard();
      app.toast(C.discarded);
      void app.refresh();
    } catch (e) {
      app.fail(e);
    }
  }

  async function applyNow() {
    try {
      app.reportApplied(await api.applyNow(), false);
      void app.refresh();
    } catch (e) {
      app.fail(e);
    }
  }
</script>

<div class="page">
  <header class="head">
    <div>
      <h1 class="headline-s">{C.title}</h1>
      <p class="body-m variant">{C.intro}</p>
    </div>
  </header>

  <div class="section-head">
    <h2 class="title-s">{C.queued}</h2>
    {#if pending.length}
      <div class="row">
        <button class="btn text danger state" type="button" onclick={discard}>{C.discardAll}</button>
        {#if running}
          <button class="btn filled with-icon state" type="button" disabled={app.restarting} onclick={() => app.restart()}>
            <Icon name="refresh" />{C.restartApply}
          </button>
        {:else}
          <button class="btn filled with-icon state" type="button" onclick={applyNow}><Icon name="check" />{C.applyNow}</button>
        {/if}
      </div>
    {/if}
  </div>

  {#if pending.length === 0}
    <p class="empty body-m variant">{C.emptyQueue}</p>
  {:else}
    <ol class="list surface-low">
      {#each pending as p (p.id)}
        <li class="item">
          <span class="lead queued"><Icon name="schedule" size={20} /></span>
          <div class="text">
            <p class="body-l">{describe(p.kind, p.names, p.description)}</p>
            <p class="body-s variant">#{p.id} · {sourceLabel(p.source)} · {when(p.created_at)}</p>
            {#if invalid.has(p.id)}
              <p class="body-s err">{C.wontWork(invalid.get(p.id) ?? "")}</p>
            {/if}
          </div>
          <button class="icon-btn state" type="button" aria-label={C.cancelChange(p.id)} title={L.common.cancel} onclick={() => cancel(p.id)}>
            <Icon name="close" />
          </button>
        </li>
      {/each}
    </ol>
  {/if}

  <div class="section-head">
    <h2 class="title-s">{C.applied}</h2>
  </div>
  {#if history.length === 0}
    <p class="empty body-m variant">{C.emptyHistory}</p>
  {:else}
    <ol class="list surface-low">
      {#each history as h (h.id)}
        <li class="item">
          <span class="lead" class:ok={h.ok} class:bad={!h.ok}><Icon name={h.ok ? "check" : "error"} size={20} /></span>
          <div class="text">
            <p class="body-l">{describe(h.kind, h.names, h.description)}</p>
            <p class="body-s variant">{sourceLabel(h.source)} · {when(h.applied_at)}</p>
            {#if !h.ok && h.message}<p class="body-s err">{h.message}</p>{/if}
          </div>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .page {
    padding: 8px 24px 48px;
    max-width: 1080px;
  }
  .head p {
    margin: 4px 0 0;
    max-width: 640px;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 48px;
    margin: 24px 0 8px;
  }
  .section-head h2 {
    color: var(--md-on-surface-variant);
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
    padding: 10px 8px 10px 16px;
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
    flex-shrink: 0;
    background: var(--md-surface-highest);
    color: var(--md-on-surface-variant);
  }
  .lead.queued {
    background: var(--md-tertiary-container);
    color: var(--md-on-tertiary-container);
  }
  .lead.bad {
    background: var(--md-error-container);
    color: var(--md-on-error-container);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .text p {
    margin: 0;
  }
  .err {
    color: var(--md-error);
  }
  .empty {
    margin: 0;
    padding: 16px 0;
  }
</style>
