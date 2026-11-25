<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../app.svelte";
  import { i18n } from "../i18n.svelte";
  import { releaseDay, updates } from "../updates.svelte";

  // The start screen's notice: the newest version and its date, and the way to the Changelog,
  // where the notes and the update are.
  const U = $derived(i18n.t.updates);
  const check = $derived(updates.check);
  // The Changelog lists every version; the card names only the newest.
  const newest = $derived(check?.pending.find((r) => r.version === check.latest) ?? check?.pending[0] ?? null);
</script>

{#if updates.showCard && check}
  <section class="card" aria-label={U.cardTitle}>
    <span class="lead" aria-hidden="true"><Icon name="download" size={20} /></span>
    <div class="body">
      <h2 class="title-m">{U.cardTitle}</h2>
      <p class="body-m sub">{U.cardText(check.latest, releaseDay(newest?.date ?? null), check.current)}</p>
    </div>
    <div class="actions">
      <button class="btn filled state" type="button" onclick={() => (app.page = "changelog")}>{U.viewChangelog}</button>
      <button class="icon-btn state" type="button" aria-label={U.dismiss} title={U.dismiss} onclick={() => updates.dismiss()}>
        <Icon name="close" size={20} />
      </button>
    </div>
  </section>
{/if}

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 16px 0 8px;
    padding: 16px 12px 16px 20px;
    border-radius: var(--shape-l);
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
  }
  .lead {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--md-primary);
    color: var(--md-on-primary);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  h2,
  .sub {
    margin: 0;
  }
  .sub {
    color: color-mix(in srgb, var(--md-on-primary-container) 78%, transparent);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    align-self: center;
  }
  .icon-btn {
    color: var(--md-on-primary-container);
  }
</style>
