<script lang="ts">
  import { createAttachmentKey } from "svelte/attachments";
  import Field from "./ui/Field.svelte";
  import TextField from "./ui/TextField.svelte";
  import Checkbox from "./ui/Checkbox.svelte";
  import { dispatch } from "./launcher.svelte";
  import type { FormBody } from "../protocol/FormBody";

  let { form }: { form: FormBody } = $props();

  /// The fields own their text while the user edits. Capturing only the
  /// seeded value is the point: the caller keys this component on the form's
  /// revision, so a reseeded form arrives as a new component.
  // svelte-ignore state_referenced_locally
  let values = $state<Record<string, string>>(
    Object.fromEntries(form.fields.map((field) => [field.id, field.value])),
  );

  function change(id: string, value: string) {
    values[id] = value;
    dispatch({ kind: "fieldChanged", id, value });
  }

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
</script>

<form
  class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4"
  onsubmit={(event) => event.preventDefault()}
>
  {#each form.fields as field, index (field.id)}
    <Field
      label={field.label}
      error={field.error}
      helpEmphasized={field.helpEmphasized}
      help={field.help ?? undefined}
    >
      {#snippet children(props)}
        {#if field.kind === "toggle"}
          <Checkbox
            {...props}
            {...{ [focusAttachment]: index === 0 ? takeFocus : nothing }}
            checked={values[field.id] === "true"}
            onCheckedChange={(checked) => change(field.id, checked ? "true" : "false")}
          />
        {:else}
          <TextField
            {...props}
            {...{ [focusAttachment]: index === 0 ? takeFocus : nothing }}
            multiline={field.kind === "template"}
            rows={field.kind === "template" ? 5 : undefined}
            type={field.kind === "password" ? "password" : "text"}
            bind:value={() => values[field.id], (value) => change(field.id, value)}
          />
        {/if}
      {/snippet}
    </Field>
  {/each}
</form>
