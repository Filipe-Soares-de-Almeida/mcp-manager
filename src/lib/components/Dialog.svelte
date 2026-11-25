<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    title,
    width = 560,
    dismissable = true,
    icon,
    children,
    actions,
  }: {
    open?: boolean;
    title: string;
    width?: number;
    dismissable?: boolean;
    icon?: Snippet;
    children: Snippet;
    actions?: Snippet;
  } = $props();

  let el: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!el) return;
    if (open && !el.open) el.showModal();
    if (!open && el.open) el.close();
  });

  function onBackdrop(e: MouseEvent) {
    if (dismissable && e.target === el) open = false;
  }

  function onCancel(e: Event) {
    if (!dismissable) e.preventDefault();
  }
</script>

<dialog
  bind:this={el}
  style:--w="{width}px"
  onclose={() => (open = false)}
  oncancel={onCancel}
  onclick={onBackdrop}
  aria-label={title}
>
  {#if open}
    <div class="dlg">
      {#if icon}<div class="icon">{@render icon()}</div>{/if}
      <h2 class="headline-s" class:centered={!!icon}>{title}</h2>
      <div class="content scroll">{@render children()}</div>
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </div>
  {/if}
</dialog>

<style>
  dialog {
    --field-bg: var(--md-surface-high);
    width: min(var(--w), calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    padding: 0;
    border: none;
    border-radius: var(--shape-xl);
    background: var(--md-surface-high);
    color: var(--md-on-surface);
    overflow: hidden;
  }
  dialog[open] {
    animation: enter 250ms var(--ease-emph);
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
    animation: fade 200ms var(--ease);
  }
  .dlg {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 48px);
    padding: 24px 0 0;
  }
  .icon {
    display: grid;
    place-items: center;
    color: var(--md-secondary);
    margin-bottom: 16px;
  }
  h2 {
    padding: 0 24px 16px;
  }
  h2.centered {
    text-align: center;
  }
  .content {
    padding: 0 24px;
    color: var(--md-on-surface-variant);
    flex: 1;
    min-height: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 20px 24px 24px;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(-12px) scale(0.98);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
