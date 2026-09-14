<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Icon from "./Icon.svelte";
  import type { FormView } from "../protocol/FormView";

  interface Props {
    view: FormView;
    onsubmit: (values: Record<string, string>) => void;
  }

  let { view, onsubmit }: Props = $props();

  type Inspection = { arguments: string[]; error: string | null };

  /// Seeded from each field's own value, so a field the user never touches
  /// still submits what it was showing. Capturing only the initial value is the
  /// point: the caller keys this component on the view, so a new form arrives as
  /// a new component rather than as a reset of this one.
  // svelte-ignore state_referenced_locally
  let values = $state<Record<string, string>>(
    Object.fromEntries(view.fields.map((field) => [field.id, field.value ?? ""])),
  );
  let inspections = $state<Record<string, Inspection>>({});

  const templateFields = $derived(view.fields.filter((field) => field.kind === "template"));

  /// The backend owns the template engine, so the frontend never parses one
  /// itself. Pure call, safe on every keystroke.
  $effect(() => {
    for (const field of templateFields) {
      const source = values[field.id] ?? "";
      invoke("inspect_template", { source }).then((result) => {
        inspections[field.id] = result as Inspection;
      });
    }
  });

  /// The launcher is summoned to be typed into, so a form that arrives without
  /// focus costs a click every time. The caret goes to the end rather than
  /// selecting, because an edit form is usually being adjusted, not replaced.
  function takeFocus(node: HTMLInputElement | HTMLTextAreaElement) {
    node.focus();
    node.setSelectionRange(node.value.length, node.value.length);
  }

  /// Stable identities. An attachment re-runs whenever its expression changes,
  /// and an inline arrow is a new function every render, which would refocus
  /// the field and jump the caret to the end on every keystroke.
  const nothing = () => {};

  function submit() {
    onsubmit(values);
  }

  /// Enter submits, except inside a template field, where it has to make a
  /// newline. There Cmd or Ctrl with Enter submits instead.
  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Enter") return;
    const inTextarea = event.target instanceof HTMLTextAreaElement;
    if (inTextarea && !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    event.stopPropagation();
    submit();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form
  class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4"
  onkeydown={onKeydown}
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  {#each view.fields as field, index (field.id)}
    <label class="flex flex-col gap-1.5">
      <span class="text-foreground-alt text-xs font-medium">{field.label}</span>
      {#if field.kind === "toggle"}
        <input
          type="checkbox"
          class="accent-foreground h-4 w-4 self-start"
          checked={values[field.id] === "true"}
          onchange={(e) => (values[field.id] = e.currentTarget.checked ? "true" : "false")}
        />
      {:else if field.kind === "template"}
        <textarea
          rows="5"
          spellcheck="false"
          class="border-border-input bg-background/60 text-foreground focus:border-foreground-alt resize-none rounded-md border px-2.5 py-2 font-mono text-sm outline-none"
          value={values[field.id] ?? ""}
          oninput={(e) => (values[field.id] = e.currentTarget.value)}
          {@attach index === 0 ? takeFocus : nothing}
        ></textarea>
        {#if inspections[field.id]?.error}
          <span class="text-destructive flex items-center gap-1.5 text-xs">
            <Icon name="circle-alert" size={12} />
            {inspections[field.id].error}
          </span>
        {:else if (inspections[field.id]?.arguments.length ?? 0) > 0}
          <span class="text-foreground-alt text-xs">
            Will ask for: {inspections[field.id].arguments.join(", ")}
          </span>
        {:else}
          <span class="text-muted-foreground text-xs">Nothing to fill in</span>
        {/if}
      {:else}
        <input
          type={field.kind === "password" ? "password" : "text"}
          spellcheck="false"
          class="border-border-input bg-background/60 text-foreground focus:border-foreground-alt rounded-md border px-2.5 py-2 text-sm outline-none"
          value={values[field.id] ?? ""}
          oninput={(e) => (values[field.id] = e.currentTarget.value)}
          {@attach index === 0 ? takeFocus : nothing}
        />
      {/if}
    </label>
  {/each}
</form>
