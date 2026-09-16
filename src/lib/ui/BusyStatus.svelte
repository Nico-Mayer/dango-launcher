<script lang="ts">
  let { message = "", visuallyHidden = false }: { message?: string; visuallyHidden?: boolean } = $props();
</script>

<span role="status" aria-live="polite" aria-atomic="true" class="min-w-0 flex-1 truncate text-busy-text">
  <span class:sr-only={visuallyHidden}>{message}</span>
</span>
{#if message}
  <span aria-hidden="true" class="dango-busy-edge"></span>
{/if}

<style>
  .dango-busy-edge {
    position: absolute;
    inset: 0 0 auto;
    height: var(--dango-busy-edge-height);
    overflow: hidden;
    pointer-events: none;
    background: var(--dango-busy-track);
  }
  .dango-busy-edge::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--dango-busy-indicator), transparent);
    animation: dango-busy-sweep var(--dango-duration-busy) linear infinite;
  }
  @keyframes dango-busy-sweep {
    from { transform: translateX(var(--dango-motion-sweep-from)); }
    to { transform: translateX(var(--dango-motion-sweep-to)); }
  }
</style>
