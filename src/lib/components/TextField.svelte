<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    value = $bindable(""),
    label,
    error = null,
    supporting = null,
    multiline = false,
    code = false,
    rows = 4,
    type = "text",
    placeholder = "",
    autofocus = false,
    leading,
    trailing,
    onkeydown,
  }: {
    value?: string;
    label: string;
    error?: string | null;
    supporting?: string | null;
    multiline?: boolean;
    code?: boolean;
    rows?: number;
    type?: string;
    placeholder?: string;
    autofocus?: boolean;
    leading?: Snippet;
    trailing?: Snippet;
    onkeydown?: (e: KeyboardEvent) => void;
  } = $props();

  const id = `tf-${Math.random().toString(36).slice(2, 9)}`;
</script>

<div class="wrap">
  <div class="field" class:error={!!error} class:float={!!placeholder} class:has-leading={!!leading} class:multiline>
    {#if leading}<span class="leading">{@render leading()}</span>{/if}
    {#if multiline}
      <textarea {id} bind:value {rows} class:code placeholder={placeholder || " "} spellcheck="false" {onkeydown}></textarea>
    {:else}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        {id}
        {type}
        bind:value
        class:code
        placeholder={placeholder || " "}
        spellcheck="false"
        autocomplete="off"
        {autofocus}
        {onkeydown}
      />
    {/if}
    <label for={id}>{label}</label>
    {#if trailing}<span class="trailing">{@render trailing()}</span>{/if}
  </div>
  {#if error}
    <div class="support err" role="alert">{error}</div>
  {:else if supporting}
    <div class="support">{supporting}</div>
  {/if}
</div>

<style>
  .wrap {
    min-width: 0;
  }
  .field {
    --bg: var(--field-bg, var(--md-surface));
    position: relative;
    display: flex;
    align-items: center;
    min-height: 56px;
    border-radius: var(--shape-xs);
    border: 1px solid var(--md-outline);
    transition: border-color 150ms var(--ease);
  }
  .field:hover {
    border-color: var(--md-on-surface);
  }
  .field:focus-within {
    border: 2px solid var(--md-primary);
  }
  .field.error,
  .field.error:hover {
    border: 2px solid var(--md-error);
  }
  input,
  textarea {
    flex: 1;
    min-width: 0;
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: var(--md-on-surface);
    font: inherit;
    font-size: 16px;
    line-height: 24px;
    letter-spacing: 0.5px;
    padding: 16px;
  }
  textarea {
    resize: vertical;
    min-height: 96px;
    padding-top: 18px;
  }
  .code {
    font-family: var(--font-code);
    font-size: 13px;
    letter-spacing: 0;
  }
  .field:focus-within input,
  .field:focus-within textarea {
    padding: 15px;
  }
  .multiline:focus-within textarea {
    padding-top: 17px;
  }
  .error input,
  .error textarea {
    padding: 15px;
  }
  input::placeholder,
  textarea::placeholder {
    color: var(--md-on-surface-variant);
    opacity: 0;
  }
  .float input::placeholder,
  .float textarea::placeholder,
  input:focus::placeholder,
  textarea:focus::placeholder {
    opacity: 0.6;
  }
  label {
    position: absolute;
    left: 12px;
    top: 16px;
    padding: 0 4px;
    color: var(--md-on-surface-variant);
    font-size: 16px;
    line-height: 24px;
    letter-spacing: 0.5px;
    pointer-events: none;
    background: var(--bg);
    transition: all 150ms var(--ease);
    max-width: calc(100% - 24px);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .has-leading label {
    left: 48px;
  }
  .has-leading input {
    padding-left: 4px;
  }
  .float label,
  input:focus + label,
  textarea:focus + label,
  input:not(:placeholder-shown) + label,
  textarea:not(:placeholder-shown) + label {
    top: -9px;
    left: 12px;
    font-size: 12px;
    line-height: 16px;
    letter-spacing: 0.4px;
  }
  .field:focus-within label {
    color: var(--md-primary);
  }
  .error label,
  .field.error:focus-within label {
    color: var(--md-error);
  }
  .leading {
    display: grid;
    place-items: center;
    padding-left: 12px;
    color: var(--md-on-surface-variant);
  }
  .trailing {
    display: flex;
    align-items: center;
    padding-right: 4px;
    color: var(--md-on-surface-variant);
  }
  .support {
    padding: 4px 16px 0;
    font-size: 12px;
    line-height: 16px;
    letter-spacing: 0.4px;
    color: var(--md-on-surface-variant);
  }
  .err {
    color: var(--md-error);
  }
</style>
