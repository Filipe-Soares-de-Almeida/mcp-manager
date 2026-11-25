<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import UpdateProgress from "../components/UpdateProgress.svelte";
  import { app } from "../app.svelte";
  import { i18n } from "../i18n.svelte";
  import { releaseDay, updates } from "../updates.svelte";
  import type { Release } from "../types";

  // Every version newest first: the ones not installed yet on top, then the history this build carries.
  const C = $derived(i18n.t.changelog);
  const U = $derived(i18n.t.updates);
  const current = $derived(app.snap?.version ?? "");
  const pending = $derived(updates.available ? (updates.check?.pending ?? []) : []);
  const history = $derived(updates.notes ?? []);
  const wall = $derived<Release[]>([...pending, ...history.filter((r) => !pending.some((p) => p.version === r.version))]);
  const busy = $derived(updates.download !== null || updates.installing);

  // Loads the notes on open, and again in the language picked in Settings.
  $effect(() => {
    void i18n.locale;
    void updates.loadNotes();
  });

  function tagOf(version: string): "available" | "installed" | null {
    if (pending.some((p) => p.version === version)) return "available";
    return version === current ? "installed" : null;
  }
</script>

<div class="page">
  <header class="head">
    <div>
      <h1 class="headline-s">{C.title}</h1>
      <p class="body-m variant">{C.sub(current)}</p>
    </div>
    <div class="check">
      {#if updates.checking}
        <span class="body-s variant">{C.checking}</span>
      {:else if updates.failed}
        <span class="body-s err">{C.checkFailed}</span>
      {:else if updates.check && !updates.available}
        <span class="body-s variant">{C.upToDate}</span>
      {/if}
      <button class="btn outlined with-icon state" type="button" disabled={updates.checking || busy} onclick={() => updates.run(true)}>
        <Icon name="refresh" />{C.check}
      </button>
    </div>
  </header>

  {#if updates.available && updates.check}
    <div class="update">
      {#if busy}
        <UpdateProgress />
      {:else}
        <p class="body-m">{C.available(updates.check.latest)}</p>
        <button class="btn filled with-icon state" type="button" onclick={() => updates.install()}>
          <Icon name="download" />{U.install(updates.check.latest)}
        </button>
      {/if}
    </div>
    {#if updates.installError}<p class="err body-s" role="alert">{U.failed(updates.installError)}</p>{/if}
  {/if}

  {#if updates.notes !== null && wall.length === 0}
    <p class="empty body-m variant">{C.empty}</p>
  {/if}

  <div class="wall">
    {#each wall as release (release.version)}
      {@const tag = tagOf(release.version)}
      <article class="version" aria-labelledby={`v-${release.version}`}>
        <header class="vh">
          <h2 class="title-m" id={`v-${release.version}`}>{C.version(release.version)}</h2>
          {#if tag}
            <span class="chip" class:live={tag === "installed"} class:queued={tag === "available"}>
              {tag === "installed" ? C.tagInstalled : C.tagAvailable}
            </span>
          {/if}
          {#if releaseDay(release.date)}<span class="day body-s variant">{releaseDay(release.date)}</span>{/if}
        </header>
        {#if release.summary}<p class="summary body-m">{release.summary}</p>{/if}
        {#each release.groups as group (group.kind)}
          <section class="group">
            <h3 class="label-l variant">{C.kinds[group.kind] ?? group.kind}</h3>
            <ul>
              {#each group.items as item, i (i)}
                <li class="body-m">{item}</li>
              {/each}
            </ul>
          </section>
        {/each}
      </article>
    {/each}
  </div>
</div>

<style>
  .page {
    padding: 8px 24px 48px;
    max-width: 880px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 16px;
  }
  .head p {
    margin: 4px 0 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .update {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 12px 12px 20px;
    margin-bottom: 16px;
    border-radius: var(--shape-l);
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
  }
  .update p {
    margin: 0;
  }
  .update :global(.progress) {
    flex: 1;
    padding: 6px 8px 6px 0;
  }
  .wall {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .version {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 24px 20px;
    border-radius: var(--shape-l);
    background: var(--md-surface-low);
  }
  .vh {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
  }
  .vh h2 {
    margin: 0;
  }
  .day {
    margin-left: auto;
  }
  .summary {
    margin: 0;
    color: var(--md-on-surface);
  }
  .group h3 {
    margin: 0 0 6px;
  }
  .group ul {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--md-on-surface);
  }
  .group li::marker {
    color: var(--md-on-surface-variant);
  }
  .err {
    color: var(--md-error);
  }
  .empty {
    margin: 0;
  }
</style>
