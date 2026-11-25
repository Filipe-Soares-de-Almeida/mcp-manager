<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../app.svelte";
  import { i18n } from "../i18n.svelte";

  const KEY = "mcp-manager.first-steps-dismissed";
  let dismissed = $state(localStorage.getItem(KEY) === "1");

  const F = $derived(i18n.t.firstSteps);
  const snap = $derived(app.snap);
  const steps = $derived(
    snap
      ? [
          { done: !!snap.self_entry?.up_to_date, ...F.register, run: () => app.registerSelf() },
          { done: snap.startup.autostart, ...F.autostart, run: () => app.setAutostart(true) },
          { done: snap.startup.shortcut, ...F.shortcut, run: () => app.createShortcut() },
        ]
      : [],
  );
  const open = $derived(!dismissed && steps.some((s) => !s.done));

  function dismiss() {
    dismissed = true;
    localStorage.setItem(KEY, "1");
  }
</script>

{#if open}
  <section class="card" aria-label={F.title}>
    <div class="head">
      <h2 class="title-m">{F.title}</h2>
      <button class="icon-btn state" type="button" aria-label={F.dismiss} onclick={dismiss}>
        <Icon name="close" size={20} />
      </button>
    </div>
    <ol>
      {#each steps as s (s.title)}
        <li class:done={s.done}>
          <span class="mark" aria-hidden="true"><Icon name={s.done ? "check" : "add"} size={18} /></span>
          <div class="text">
            <p class="title-s">{s.title}</p>
            <p class="body-s variant">{s.text}</p>
          </div>
          {#if !s.done}
            <button class="btn tonal state" type="button" onclick={s.run}>{s.action}</button>
          {/if}
        </li>
      {/each}
    </ol>
  </section>
{/if}

<style>
  .card {
    margin: 16px 0 8px;
    padding: 12px 8px 12px 20px;
    border-radius: var(--shape-l);
    background: var(--md-surface-container);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  ol {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  li {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 12px 8px 0;
  }
  .mark {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border: 1px solid var(--md-outline);
    color: var(--md-on-surface-variant);
  }
  .done .mark {
    border-color: transparent;
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
  }
  .done .title-s {
    color: var(--md-on-surface-variant);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .text p {
    margin: 0;
  }
</style>
