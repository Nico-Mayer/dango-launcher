<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import Icon, { isNamedIcon, namedIcon } from "./Icon.svelte";
  import { highlight } from "./highlight";
  import { tintClasses } from "./tint";

  interface Props {
    title: string;
    subtitle?: string | null;
    /// Either `icon:<name>` for a named icon or a path to an image file.
    icon?: string | null;
    /// Character offsets in the title that matched the query. Empty where the
    /// list does not rank, such as a view a command pushed.
    matchPositions?: number[];
    /// The colour the contributing extension named, drawn behind a named icon
    /// so results from different extensions are told apart at a glance.
    tint?: string | null;
    /// Whether the icon is content in its own right, as a copied image is,
    /// rather than a symbol standing for the row. Content gets a frame and a
    /// little more room, because one screenshot has to be told from another.
    iconIsContent?: boolean;
  }

  let {
    title,
    subtitle = null,
    icon = null,
    matchPositions = [],
    iconIsContent = false,
    tint = null,
  }: Props = $props();

  const named = $derived(namedIcon(icon));
  const file = $derived(icon && !isNamedIcon(icon) ? convertFileSrc(icon) : null);
  const tinted = $derived(tintClasses(tint));
</script>

{#if named}
  <div
    class="flex size-icon-tile shrink-0 items-center justify-center rounded-md {tinted ||
      'text-foreground-alt'}"
  >
    <Icon name={named} size={20} class="size-icon-result" />
  </div>
{:else if file}
  <img
    src={file}
    alt=""
    class="size-icon-tile shrink-0 {iconIsContent
      ? 'border-border-card rounded-sm border-edge object-cover'
      : 'object-contain'}"
  />
{:else}
  <div class="bg-muted size-icon-tile shrink-0 rounded-sm"></div>
{/if}

<div class="flex min-w-0 items-baseline gap-2">
  <span class="text-foreground [[data-selected]_&]:text-selection-text truncate text-sm">
    {#each highlight(title, matchPositions) as segment, index (index)}
      <span class={segment.matched ? "font-semibold" : ""}>{segment.text}</span>
    {/each}
  </span>
  {#if subtitle}
    <span class="text-muted-foreground truncate text-xs">{subtitle}</span>
  {/if}
</div>
