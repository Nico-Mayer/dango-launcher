<script lang="ts">
  import BusyStatus from "../ui/BusyStatus.svelte";
  import Shortcut from "../ui/Shortcut.svelte";
  import type { Snippet } from "svelte";

  let { primaryLabel, confirmKey = "↵", actions, status = "", hideStatus = false }: { primaryLabel: string; confirmKey?: string; actions?: Snippet; status?: string; hideStatus?: boolean } = $props();
</script>

<footer class="border-border-card text-muted-foreground relative flex h-footer shrink-0 items-center gap-4 border-t-edge px-4 text-xs">
  <span class="text-foreground-alt shrink-0 font-medium">Dango</span>
  <span class="bg-tint-amber text-tint-amber-foreground rounded-keycap -ml-2.5 hidden shrink-0 px-1.5 font-medium in-data-development:inline">dev</span>
  <BusyStatus message={status} visuallyHidden={hideStatus} />
  <div class="flex min-w-0 max-w-1/2 items-center gap-4">
    <span class="flex min-w-0 items-center gap-1.5">
      <span class="truncate">{primaryLabel}</span>
      <Shortcut>{confirmKey}</Shortcut>
    </span>
    {@render actions?.()}
  </div>
</footer>
