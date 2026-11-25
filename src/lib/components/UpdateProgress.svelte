<script lang="ts">
  import { i18n } from "../i18n.svelte";
  import { megabytes, updates } from "../updates.svelte";

  const U = $derived(i18n.t.updates);
  const d = $derived(updates.download);
  const share = $derived(d?.total ? Math.min(1, d.done / d.total) : 0);
  const text = $derived(
    updates.installing
      ? U.installing
      : d?.total
        ? U.downloading(megabytes(d.done), megabytes(d.total))
        : U.downloadingUnknown(megabytes(d?.done ?? 0)),
  );
</script>

{#if d}
  <div class="progress" role="status">
    <div class="bar" role="progressbar" aria-label={text} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(share * 100)}>
      <i class:indeterminate={!d.total} style:transform={d.total ? `translateX(${(share - 1) * 100}%)` : undefined}></i>
    </div>
    <p class="body-s">{text}</p>
  </div>
{/if}

<style>
  .progress {
    display: grid;
    gap: 6px;
  }
  .bar {
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--md-surface-highest);
  }
  .bar i {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--md-primary);
    transition: transform 250ms var(--ease);
  }
  .bar i.indeterminate {
    width: 40%;
    animation: slide 1.2s var(--ease) infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(250%);
    }
  }
  p {
    margin: 0;
    color: var(--md-on-surface-variant);
  }
</style>
