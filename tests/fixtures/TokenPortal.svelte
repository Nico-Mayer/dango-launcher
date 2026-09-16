<script lang="ts">
  import { Portal } from "bits-ui";
</script>

<Portal>
  <div data-token-portal class="bg-background-alt text-foreground rounded-button" aria-hidden="true">
    <span data-token-selection class="bg-selection-bg text-selection-text">Actions</span>
    <span data-token-hover class="bg-hover-bg">Actions</span>
    <span data-token-focus class="border-focus-ring border">Search</span>
    <span data-token-busy class="text-busy-text">Loading…</span>
    <span data-token-indicator class="bg-busy-indicator"></span>
  </div>
</Portal>
