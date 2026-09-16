<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createAttachmentKey } from "svelte/attachments";
  import Field from "./ui/Field.svelte";
  import TextField from "./ui/TextField.svelte";
  import Checkbox from "./ui/Checkbox.svelte";
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
  function takeFocus(node: HTMLInputElement | HTMLTextAreaElement | HTMLButtonElement) {
    node.focus();
    if (node instanceof HTMLInputElement || node instanceof HTMLTextAreaElement)
      node.setSelectionRange(node.value.length, node.value.length);
  }

  /// Stable identities. An attachment re-runs whenever its expression changes,
  /// and an inline arrow is a new function every render, which would refocus
  /// the field and jump the caret to the end on every keystroke.
  const nothing = () => {};
  const focusAttachment = createAttachmentKey();

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
    <Field
      label={field.label}
      error={field.kind === "template" ? inspections[field.id]?.error : undefined}
      helpEmphasized={(inspections[field.id]?.arguments.length ?? 0) > 0}
      help={field.kind !== "template" ? undefined : (inspections[field.id]?.arguments.length ?? 0) > 0
        ? `Will ask for: ${inspections[field.id].arguments.join(", ")}`
        : "Nothing to fill in"}
    >
      {#snippet children(props)}
        {#if field.kind === "toggle"}
          <Checkbox
            {...props}
            {...{ [focusAttachment]: index === 0 ? takeFocus : nothing }}
            checked={values[field.id] === "true"}
            onCheckedChange={(checked) => (values[field.id] = checked ? "true" : "false")}
          />
        {:else}
          <TextField
            {...props}
            {...{ [focusAttachment]: index === 0 ? takeFocus : nothing }}
            multiline={field.kind === "template"}
            rows={field.kind === "template" ? 5 : undefined}
            type={field.kind === "password" ? "password" : "text"}
            bind:value={values[field.id]}
          />
        {/if}
      {/snippet}
    </Field>
  {/each}
</form>
