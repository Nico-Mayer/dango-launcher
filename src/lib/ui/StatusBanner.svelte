<script lang="ts">
  import Icon from "../Icon.svelte";

  let { message }: { message: string } = $props();
</script>

<div class="border-border-card text-destructive flex shrink-0 items-center gap-2 border-t-edge px-5 py-2 text-sm wrap-anywhere">
  <Icon name="circle-alert" size={16} class="size-icon-status" />
  {message}
</div>
