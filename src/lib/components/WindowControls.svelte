<script lang="ts">
  import { onMount } from "svelte";
  import { appWindow } from "../api";
  import { i18n } from "../i18n.svelte";

  let maximized = $state(false);
  const W = $derived(i18n.t.window);

  onMount(() => {
    const sync = async () => (maximized = await appWindow.isMaximized());
    void sync();
    const off = appWindow.onResized(() => void sync());
    return () => void off.then((f) => f());
  });
</script>

<!-- Thin 10 px glyphs drawn like the system's, so the buttons read as window controls. -->
<div class="controls">
  <button type="button" aria-label={W.minimize} title={W.minimize} onclick={() => appWindow.minimize()}>
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5.5h10" /></svg>
  </button>
  <button
    type="button"
    aria-label={maximized ? W.restore : W.maximize}
    title={maximized ? W.restore : W.maximize}
    onclick={() => appWindow.toggleMaximize()}
  >
    {#if maximized}
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" /></svg>
    {:else}
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5 .5h9v9h-9z" /></svg>
    {/if}
  </button>
  <button type="button" class="close" aria-label={W.close} title={W.close} onclick={() => appWindow.close()}>
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5 .5l9 9M9.5 .5l-9 9" /></svg>
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-self: stretch;
  }
  button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--md-on-surface-variant);
    cursor: default;
    transition: background 120ms var(--ease), color 120ms var(--ease);
  }
  button:hover {
    background: var(--md-surface-high);
    color: var(--md-on-surface);
  }
  button:active {
    background: var(--md-surface-highest);
  }
  /* The red close hover is the platform's convention, not a state color. */
  .close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .close:active {
    background: #b22a1d;
  }
  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
</style>
