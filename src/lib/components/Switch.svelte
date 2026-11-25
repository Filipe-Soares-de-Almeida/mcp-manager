<script lang="ts">
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: { checked: boolean; label: string; disabled?: boolean; onchange: (value: boolean) => void } = $props();
</script>

<button
  type="button"
  role="switch"
  class="switch"
  class:on={checked}
  aria-checked={checked}
  aria-label={label}
  title={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="handle">
    {#if checked}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" /></svg>
    {/if}
  </span>
</button>

<style>
  .switch {
    position: relative;
    flex-shrink: 0;
    width: 52px;
    height: 32px;
    padding: 0;
    border-radius: 16px;
    border: 2px solid var(--md-outline);
    background: var(--md-surface-highest);
    cursor: pointer;
    transition: background 200ms var(--ease), border-color 200ms var(--ease);
  }
  .handle {
    position: absolute;
    top: 50%;
    left: 6px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--md-outline);
    transform: translateY(-50%);
    display: grid;
    place-items: center;
    transition: all 250ms var(--ease-emph);
  }
  .handle::before {
    content: "";
    position: absolute;
    inset: -12px;
    border-radius: 50%;
    background: var(--md-on-surface);
    opacity: 0;
    transition: opacity 150ms var(--ease);
  }
  .switch:hover .handle::before {
    opacity: 0.08;
  }
  .switch:hover .handle {
    background: var(--md-on-surface-variant);
  }
  .on {
    background: var(--md-primary);
    border-color: var(--md-primary);
  }
  .on .handle {
    left: 22px;
    width: 24px;
    height: 24px;
    background: var(--md-on-primary);
  }
  .on:hover .handle {
    background: var(--md-primary-container);
  }
  .on:hover .handle::before {
    background: var(--md-primary);
  }
  .handle svg {
    width: 16px;
    height: 16px;
    fill: var(--md-on-primary-container);
    position: relative;
  }
  .switch:active .handle {
    width: 28px;
    height: 28px;
    left: 2px;
  }
  .on:active .handle {
    left: 18px;
  }
  .switch:disabled {
    opacity: 0.38;
    cursor: default;
  }
</style>
