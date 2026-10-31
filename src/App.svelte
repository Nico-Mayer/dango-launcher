<script lang="ts">
    import { listen } from "@tauri-apps/api/event";
    import * as Command from "./lib/ui/command";
    import { onMount, tick } from "svelte";
    import ActionPanel from "./lib/ActionPanel.svelte";
    import LauncherFrame from "./lib/launcher/LauncherFrame.svelte";
    import LauncherFooter from "./lib/launcher/LauncherFooter.svelte";
    import EmptyMessage from "./lib/ui/EmptyMessage.svelte";
    import StatusBanner from "./lib/ui/StatusBanner.svelte";
    import { pointerActive } from "./lib/input.svelte";
    import { boundChord, dispatch, echoed } from "./lib/launcher.svelte";
    import ProtocolView from "./lib/ProtocolView.svelte";
    import ResultRow from "./lib/ResultRow.svelte";
    import type { Focus } from "./protocol/Focus";
    import type { ResultItem } from "./protocol/ResultItem";
    import type { Snapshot } from "./protocol/Snapshot";

    let snapshot = $state.raw<Snapshot | null>(null);
    let panelPresent = $state(false);

    const root = $derived(snapshot?.screen.kind === "root" ? snapshot.screen : null);
    const query = echoed(() => root?.query ?? { text: "", revision: -1 });

    function surface(focus: Focus) {
        return document.querySelector<HTMLElement>(`[data-surface="${focus}"]`);
    }

    /// Capture on the document, so a bound key is taken before Bits sees it:
    /// selection, Enter, and Escape belong to the state machine. The action
    /// panel's exit guard listens on the window and runs before this.
    function onKeydown(event: KeyboardEvent) {
        if (!snapshot) return;
        // A focused button answers Enter and Space itself, as the Actions
        // trigger does by opening the panel.
        const native = event.target instanceof Element && event.target.closest("[data-native-keys]");
        if (native && (event.key === "Enter" || event.key === " ")) return;
        const chord = boundChord(snapshot.bindings, event);
        if (!chord) return;
        event.preventDefault();
        event.stopPropagation();
        dispatch({ kind: "key", chord });
    }

    onMount(() => {
        const updateVisibility = () => {
            document.documentElement.toggleAttribute("data-dango-hidden", document.hidden);
        };
        updateVisibility();
        document.addEventListener("visibilitychange", updateVisibility);
        document.addEventListener("keydown", onKeydown, true);
        // Ready runs while the window is still parked offscreen, so the first
        // activation already has the frecent items and their icons painted.
        const unlisten = listen<Snapshot>("dango://state", (event) => {
            snapshot = event.payload;
        });
        unlisten.then(() => dispatch({ kind: "ready" }));
        return () => {
            document.removeEventListener("visibilitychange", updateVisibility);
            document.removeEventListener("keydown", onKeydown, true);
            document.documentElement.removeAttribute("data-dango-hidden");
            unlisten.then((un) => un());
        };
    });

    let seenActivation = 0;
    $effect(() => {
        if (!snapshot || snapshot.activation.count === seenActivation) return;
        seenActivation = snapshot.activation.count;
        if (snapshot.focus === "rootInput") {
            const input = surface("rootInput") as HTMLInputElement | null;
            input?.focus();
            input?.select();
        }
        const probe = snapshot.activation.probe;
        if (probe !== null)
            requestAnimationFrame(() =>
                requestAnimationFrame(() => dispatch({ kind: "painted", probe })),
            );
    });

    /// Places the cursor where the snapshot says the keyboard is, whenever
    /// that moves. A form places its own on mount, and the action panel hands
    /// focus back to where it was opened from, unless that went away with it.
    let lastFocus: Focus | null = null;
    $effect(() => {
        const focus = snapshot?.focus;
        if (!focus) return;
        const previous = lastFocus;
        lastFocus = focus;
        if (focus === previous || ["panel", "form", "nothing"].includes(focus)) return;
        tick().then(() => {
            const active = document.activeElement;
            if (previous === "panel" && active && active !== document.body) return;
            surface(focus)?.focus();
        });
    });
</script>

<svelte:window onblur={() => dispatch({ kind: "blur" })} />

<!-- The row is not focusable, so a click would otherwise move focus off the
     prompt and leave the launcher unable to type. The scroll margin keeps a
     group's heading in view when arrowing up lands on its first row. -->
{#snippet row(item: ResultItem)}
    <Command.Item
        value={item.id}
        onSelect={() => !panelPresent && dispatch({ kind: "activate", id: item.id })}
        onmousedown={(event) => event.preventDefault()}
        class="first:scroll-mt-7"
    >
        <ResultRow
            title={item.title}
            subtitle={item.subtitle}
            icon={item.icon}
            matchPositions={item.matchPositions}
            tint={item.tint}
        />
    </Command.Item>
{/snippet}

{#if snapshot?.screen.kind === "error"}
    <LauncherFrame development={snapshot.development} class="justify-center gap-2 px-6">
        <span class="text-foreground text-sm"
            >This view needs a newer version of Dango.</span
        >
        <span class="text-muted-foreground text-xs"
            >Press Escape to go back.</span
        >
    </LauncherFrame>
{:else if snapshot?.screen.kind === "view"}
    <LauncherFrame development={snapshot.development}>
        <!-- Fills the space so the footer sits on the bottom edge rather than
         directly under a short form. -->
        <div class="flex min-h-0 flex-1 flex-col">
            <ProtocolView {snapshot} screen={snapshot.screen}>
                {#snippet beforeFooter()}
                    {#if snapshot?.failure}
                        <StatusBanner message={snapshot.failure} />
                    {/if}
                {/snippet}
            </ProtocolView>
        </div>
    </LauncherFrame>
{:else if snapshot && root}
    <Command.Root bind:value={() => root.selected ?? "", () => {}}>
        {#snippet child({ props })}
        <LauncherFrame {...props} development={snapshot?.development}>
        <Command.Input
            data-surface="rootInput"
            bind:value={
                () => query.text,
                (text) => {
                    query.text = text;
                    dispatch({ kind: "queryChanged", text });
                }
            }
            placeholder="Search apps and commands"
        />
        <!-- The inset lives outside the scroller, so the gap above the first row
         and below the last one is there at every scroll position instead of
         appearing only at the two ends. -->
        <div
            aria-busy={root.busy}
            class="border-border-card flex min-h-0 flex-1 flex-col border-t-edge py-2"
        >
            <Command.List data-pointer={pointerActive() ? "" : undefined}>
                <Command.Viewport>
                    {#each root.sections as section (section.heading)}
                        {#if section.heading}
                            <Command.Group>
                                <Command.GroupHeading>
                                    {section.heading}
                                </Command.GroupHeading>
                                <Command.GroupItems>
                                    {#each section.items as item (item.id)}
                                        {@render row(item)}
                                    {/each}
                                </Command.GroupItems>
                            </Command.Group>
                        {:else}
                            {#each section.items as item (item.id)}
                                {@render row(item)}
                            {/each}
                        {/if}
                    {/each}
                    {#if root.empty}
                        <EmptyMessage class="truncate">{root.empty}</EmptyMessage>
                    {/if}
                </Command.Viewport>
            </Command.List>
        </div>

        {#if snapshot?.failure}
            <StatusBanner message={snapshot.failure} />
        {/if}

        <LauncherFooter primaryLabel={snapshot?.footer.primaryLabel ?? ""} status={snapshot?.footer.status}>
            {#snippet actions()}
                {#if snapshot}
                    <ActionPanel
                        panel={snapshot.panel}
                        bind:present={panelPresent}
                        returnFocus={() => surface("rootInput")}
                    />
                {/if}
            {/snippet}
        </LauncherFooter>
        </LauncherFrame>
        {/snippet}
    </Command.Root>
{/if}
