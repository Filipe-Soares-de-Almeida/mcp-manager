<script lang="ts">
  import { untrack } from "svelte";
  import Dialog from "./Dialog.svelte";
  import TextField from "./TextField.svelte";
  import Icon from "./Icon.svelte";
  import { api } from "../api";
  import { app } from "../app.svelte";
  import { i18n } from "../i18n.svelte";
  import { fromForm, parsePasted, toForm, type Errors, type Form } from "../spec";
  import type { Row } from "../types";

  const L = $derived(i18n.t);
  const E = $derived(i18n.t.editor);

  let {
    open = $bindable(false),
    row = null,
    taken = [],
  }: { open?: boolean; row?: Row | null; taken?: string[] } = $props();

  let form = $state<Form>(toForm("", null));
  let errors = $state<Errors>({});
  let reveal = $state<boolean[]>([]);
  let pasteOpen = $state(false);
  let pasteText = $state("");
  let advanced = $state(false);
  let previewOpen = $state(false);
  let saving = $state(false);
  let editing = $state<Row | null>(null);

  // Reset only when the dialog opens: the list refreshes while it is open,
  // and that must not wipe what is being typed.
  $effect(() => {
    if (open)
      untrack(() => {
        editing = row;
        form = toForm(row?.name ?? "", row?.spec ?? null);
        errors = {};
        reveal = form.env.map(() => false);
        pasteOpen = !row;
        pasteText = "";
        advanced = !!form.extra;
        previewOpen = false;
      });
  });

  const preview = $derived.by(() => {
    const name = form.name || E.previewName;
    const { spec } = fromForm({ ...form, name }, [], null);
    return spec ? JSON.stringify({ [name]: spec }, null, 2) : null;
  });

  function applyPaste() {
    try {
      const { name, spec } = parsePasted(pasteText);
      form = toForm(name ?? form.name, spec);
      reveal = form.env.map(() => false);
      advanced = !!form.extra;
      errors = {};
      pasteOpen = false;
    } catch {
      errors = { ...errors, paste: E.pasteError };
    }
  }

  async function save() {
    const original = editing?.name ?? null;
    const result = fromForm(form, taken, original);
    errors = result.errors;
    if (!result.spec) return;
    saving = true;
    try {
      const name = form.name;
      const o = editing
        ? await api.update(editing.name, name !== editing.name ? name : null, result.spec)
        : await api.add(name, result.spec);
      open = false;
      app.outcome(o, editing ? E.edited(name) : E.added(name));
    } catch (e) {
      app.fail(e);
    } finally {
      saving = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void save();
  }
</script>

<Dialog bind:open title={editing ? E.titleEdit(editing.name) : E.titleAdd} width={720} dismissable={false}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="form" onkeydown={onKey}>
    {#if editing && !editing.enabled}
      <p class="note body-m">
        <Icon name="info" size={20} />
        {E.disabledNote}
      </p>
    {/if}

    {#if !editing}
      {#if pasteOpen}
        <section class="paste">
          <div class="paste-head">
            <span class="title-s">{E.pasteTitle}</span>
            <button class="icon-btn state" type="button" aria-label={L.common.close} onclick={() => (pasteOpen = false)}>
              <Icon name="close" size={20} />
            </button>
          </div>
          <p class="body-m variant">{E.pasteText}</p>
          <TextField
            label="JSON"
            multiline
            code
            rows={5}
            bind:value={pasteText}
            placeholder={'"github": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"] }'}
            error={errors.paste}
          />
          <div class="row-end">
            <button class="btn tonal" type="button" disabled={!pasteText.trim()} onclick={applyPaste}>{E.pasteFill}</button>
          </div>
        </section>
      {:else}
        <button class="btn text with-icon state paste-toggle" type="button" onclick={() => (pasteOpen = true)}>
          <Icon name="paste" /> {E.pasteToggle}
        </button>
      {/if}
    {/if}

    <TextField label={E.name} bind:value={form.name} error={errors.name} supporting={E.nameHint} autofocus={!!editing} />
    <TextField label={E.command} code bind:value={form.command} error={errors.command} supporting={E.commandHint} />

    <section>
      <h3 class="title-s">{E.args}</h3>
      {#each form.args as _, i (i)}
        <div class="line">
          <TextField label={E.arg(i + 1)} code bind:value={form.args[i]} />
          <button
            class="icon-btn state"
            type="button"
            aria-label={E.removeArg(i + 1)}
            onclick={() => (form.args = form.args.filter((_, j) => j !== i))}
          >
            <Icon name="remove" />
          </button>
        </div>
      {/each}
      <button class="btn text with-icon state" type="button" onclick={() => (form.args = [...form.args, ""])}>
        <Icon name="add" /> {E.addArg}
      </button>
    </section>

    <section>
      <h3 class="title-s">{E.env}</h3>
      {#each form.env as entry, i (i)}
        <div class="line env">
          <TextField label={E.varName} code bind:value={entry.key} />
          <TextField label={E.varValue} code type={reveal[i] ? "text" : "password"} bind:value={entry.value}>
            {#snippet trailing()}
              <button
                class="icon-btn state"
                type="button"
                aria-label={reveal[i] ? E.hideValue : E.showValue}
                onclick={() => (reveal[i] = !reveal[i])}
              >
                <Icon name={reveal[i] ? "visibilityOff" : "visibility"} size={20} />
              </button>
            {/snippet}
          </TextField>
          <button
            class="icon-btn state"
            type="button"
            aria-label={E.removeVar(entry.key)}
            onclick={() => {
              form.env = form.env.filter((_, j) => j !== i);
              reveal = reveal.filter((_, j) => j !== i);
            }}
          >
            <Icon name="remove" />
          </button>
        </div>
      {/each}
      {#if errors.env}<p class="err body-s" role="alert">{errors.env}</p>{/if}
      <button
        class="btn text with-icon state"
        type="button"
        onclick={() => {
          form.env = [...form.env, { key: "", value: "" }];
          reveal = [...reveal, true];
        }}
      >
        <Icon name="add" /> {E.addVar}
      </button>
    </section>

    <section>
      <button class="disclosure state" type="button" aria-expanded={advanced} onclick={() => (advanced = !advanced)}>
        <span class="title-s">{E.extra}</span>
        <span class="body-s variant">{E.extraHint}</span>
      </button>
      {#if advanced}
        <TextField
          label="JSON"
          multiline
          code
          rows={4}
          bind:value={form.extra}
          placeholder={E.extraPlaceholder}
          error={errors.extra}
          supporting={E.extraSupport}
        />
      {/if}
    </section>

    <section>
      <button class="disclosure state" type="button" aria-expanded={previewOpen} onclick={() => (previewOpen = !previewOpen)}>
        <span class="title-s">{E.preview}</span>
        <span class="body-s variant">{E.previewHint}</span>
      </button>
      {#if previewOpen}
        <pre class="preview code selectable">{preview ?? E.previewEmpty}</pre>
      {/if}
    </section>
  </div>

  {#snippet actions()}
    <button class="btn text state" type="button" onclick={() => (open = false)}>{L.common.cancel}</button>
    <button class="btn filled state" type="button" disabled={saving} onclick={save}>
      {editing ? L.common.save : L.common.add}
    </button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 8px 0 4px;
    color: var(--md-on-surface);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  section > .btn {
    align-self: flex-start;
    margin-left: -12px;
  }
  h3 {
    color: var(--md-on-surface-variant);
  }
  .line {
    display: grid;
    grid-template-columns: 1fr 40px;
    gap: 8px;
    align-items: start;
  }
  .line > .icon-btn {
    margin-top: 8px;
  }
  .line.env {
    grid-template-columns: minmax(140px, 0.8fr) 1.2fr 40px;
  }
  .paste {
    background: var(--md-surface-highest);
    border-radius: var(--shape-l);
    padding: 12px 16px 16px;
    --field-bg: var(--md-surface-highest);
  }
  .paste-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-right: -8px;
  }
  .paste p {
    margin: 0;
  }
  .paste-toggle {
    align-self: flex-start;
    margin-left: -12px;
  }
  .row-end {
    display: flex;
    justify-content: flex-end;
  }
  .note {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin: 0;
    padding: 12px 16px;
    border-radius: var(--shape-m);
    background: var(--md-surface-highest);
    color: var(--md-on-surface-variant);
  }
  .note :global(svg) {
    flex-shrink: 0;
    color: var(--md-secondary);
  }
  .disclosure {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 8px 12px;
    margin: 0 -12px;
    border: none;
    border-radius: var(--shape-s);
    background: transparent;
    color: var(--md-on-surface);
    font: inherit;
    text-align: left;
  }
  .disclosure .title-s::after {
    content: " ▾";
    color: var(--md-on-surface-variant);
  }
  .disclosure[aria-expanded="true"] .title-s::after {
    content: " ▴";
  }
  .preview {
    margin: 0;
    padding: 12px 16px;
    border-radius: var(--shape-m);
    background: var(--md-surface-lowest);
    color: var(--md-on-surface);
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 240px;
    overflow: auto;
  }
  .err {
    color: var(--md-error);
    margin: 0;
  }
</style>
