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
  class="flex flex-col gap-3 px-4 py-3"
  onkeydown={onKeydown}
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  {#each view.fields as field (field.id)}
    <label class="flex flex-col gap-1">
      <span class="text-foreground-alt text-xs">{field.label}</span>
      {#if field.kind === "toggle"}
        <input
          type="checkbox"
          checked={values[field.id] === "true"}
          onchange={(e) => (values[field.id] = e.currentTarget.checked ? "true" : "false")}
        />
      {:else if field.kind === "template"}
        <textarea
          rows="4"
          class="border-border-input bg-background/60 text-foreground rounded-md border px-2 py-1 font-mono text-sm focus:outline-none"
          value={values[field.id] ?? ""}
          oninput={(e) => (values[field.id] = e.currentTarget.value)}
        ></textarea>
        {#if inspections[field.id]?.error}
          <span class="text-destructive flex items-center gap-1 text-xs">
            <Icon name="circle-alert" size={12} />
            {inspections[field.id].error}
          </span>
        {:else if (inspections[field.id]?.arguments.length ?? 0) > 0}
          <span class="text-foreground-alt text-xs">
            Will ask for: {inspections[field.id].arguments.join(", ")}
          </span>
        {:else}
          <span class="text-muted-foreground text-xs">Asks for nothing</span>
        {/if}
      {:else}
        <input
          type={field.kind === "password" ? "password" : "text"}
          class="border-border-input bg-background/60 text-foreground rounded-md border px-2 py-1 text-sm focus:outline-none"
          value={values[field.id] ?? ""}
          oninput={(e) => (values[field.id] = e.currentTarget.value)}
        />
      {/if}
    </label>
  {/each}
</form>
