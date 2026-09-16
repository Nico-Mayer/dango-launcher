<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import * as Command from "./lib/ui/command";
    import { onMount } from "svelte";
    import ActionPanel from "./lib/ActionPanel.svelte";
    import LauncherFrame from "./lib/launcher/LauncherFrame.svelte";
    import LauncherFooter from "./lib/launcher/LauncherFooter.svelte";
    import EmptyMessage from "./lib/ui/EmptyMessage.svelte";
    import StatusBanner from "./lib/ui/StatusBanner.svelte";
    import { inUserGesture, pointerActive } from "./lib/input.svelte";
    import ProtocolView from "./lib/ProtocolView.svelte";
    import ResultRow from "./lib/ResultRow.svelte";
    import {
        matchesShortcut,
        type ActionResponse,
        type RenderPayload,
        type ResultItem,
        type ResultsPayload,
    } from "./lib/types";
    import type { ViewTree } from "./protocol/ViewTree";

    const PROTOCOL_VERSION = 1;

    let query = $state("");
    let results = $state<ResultItem[]>([]);
    /// What the keyboard or a click last picked. The selection is derived from
    /// it rather than stored, so results arriving without the picked item fall
    /// back to the first row instead of leaving the list with nothing selected.
    let pickedId = $state("");
    let stack = $state<ViewTree[]>([]);
    let panelOpen = $state(false);
    let panelPresent = $state(false);
    let protocolError = $state(false);
    /// The extension whose command pushed what is on the stack, so an action
    /// chosen inside its view goes back to it.
    let viewOwner = $state<string | null>(null);
    /// The run whose tree is on top of the stack. A streaming command replaces
    /// its own view as the answer grows, so a tree from the run already showing
    /// takes the place of the one there rather than pushing another.
    let shownInvocation: number | null = null;
    let failure = $state<string | null>(null);
    // The title of a command that has been invoked and has neither finished nor
    // shown a view yet.
    let workingTitle = $state<string | null>(null);
    let inputEl = $state<HTMLInputElement | null>(null);

    // The query whose results we will display; a late event for an older query is
    // dropped so cancelled results never show.
    let liveQuery = "";

    /// The same idea for views. Cancelling a streaming command does not recall the
    /// tree it has already emitted, so one more can land after the user has left:
    /// with nothing on the stack to replace it goes on as a new view, and the
    /// answer they walked away from is back on screen, frozen part-written.
    /// Invocation ids only ever go up, so one high-water mark covers every run at
    /// or before the one abandoned.
    let lastInvocation = -1;
    let abandonedInvocation = -1;

    /// Leaves whatever is showing, and with it anything still arriving for it.
    function abandonShownView() {
        abandonedInvocation = lastInvocation;
        shownInvocation = null;
    }

    const selectedId = $derived(
        results.some((r) => r.id === pickedId)
            ? pickedId
            : (results[0]?.id ?? ""),
    );
    const selectedItem = $derived(results.find((r) => r.id === selectedId));
    const suggestions = $derived(results.filter((r) => r.suggested));
    const everythingElse = $derived(results.filter((r) => !r.suggested));

    function runSearch(q: string) {
        liveQuery = q;
        invoke("search", { query: q });
    }

    /// A failure deliberately survives this.
    ///
    /// An action that pastes hides the launcher before it does the work, so it
    /// can only report a failure once the window is already gone, and whether
    /// that report arrives before or after this reset is a race. Clearing it here
    /// lost the reason perhaps half the time, which is how a broken paste came to
    /// look like nothing happening at all. The message is carried to the next
    /// activation instead, and cleared as soon as the user types, acts, or
    /// presses Escape.
    function resetToRoot() {
        query = "";
        results = [];
        stack = [];
        viewOwner = null;
        // Deliberately not abandoning: `hide_for_work` resets through here too, on
        // behalf of a command that is still wanted and whose next view is what
        // brings the launcher back.
        shownInvocation = null;
        panelOpen = false;
        protocolError = false;
        workingTitle = null;
        runSearch("");
    }

    async function runAction(item: ResultItem, actionId: string | undefined) {
        if (!actionId) return;
        failure = null;
        panelOpen = false;
        const response = (await invoke("run_action", {
            extensionId: item.extensionId,
            itemId: item.id,
            actionId,
            values: null,
        })) as ActionResponse;
        if (response.kind === "copy") {
            await navigator.clipboard.writeText(response.text);
            invoke("dismiss");
        } else if (response.kind === "replaced") {
            // A root action that produces a view opens it. A snippet needing
            // arguments has to ask before it can do anything.
            viewOwner = item.extensionId;
            stack = [response.tree];
            shownInvocation = null;
        } else if (response.kind === "removed") {
            runSearch(query);
        } else if (response.kind === "failed") {
            failure = response.message;
        }
        // A plain success already hid the launcher in the backend.
    }

    /// An action chosen inside a pushed view goes to the extension that owns the
    /// running command, through the same path a root result uses.
    async function runViewAction(
        actionId: string,
        itemId: string | null,
        values?: Record<string, string>,
    ) {
        if (!viewOwner) return;
        failure = null;
        const response = (await invoke("run_action", {
            extensionId: viewOwner,
            itemId: itemId ?? "",
            actionId,
            values: values ?? null,
        })) as ActionResponse;
        if (response.kind === "copy") {
            await navigator.clipboard.writeText(response.text);
            invoke("dismiss");
        } else if (
            response.kind === "replaced" ||
            response.kind === "removed"
        ) {
            stack = [...stack.slice(0, -1), response.tree];
            shownInvocation = null;
        } else if (response.kind === "failed") {
            failure = response.message;
        }
        // "started" leaves the view where it is; what it started arrives on the
        // render channel and pushes its own.
    }

    /// Confirming a result acts on it: a command is invoked, anything else runs
    /// its primary action. A command carries no actions, which is what tells the
    /// two apart.
    async function confirm(item: ResultItem | undefined) {
        if (!item) return;
        if (item.actions.length === 0) {
            failure = null;
            panelOpen = false;
            workingTitle = item.title;
            // The view that follows carries its own owner, so nothing is read back
            // here; only a refusal to start the command needs handling.
            await invoke("invoke_command", { commandId: item.id }).catch(
                (reason) => {
                    failure = String(reason);
                    workingTitle = null;
                },
            );
            return;
        }
        runAction(item, item.actions[0].id);
    }

    // Keys the Command primitive does not own: Escape's two-stage dismiss, the
    // action panel, and per-action shortcuts. Arrow and Enter navigation and
    // scroll-into-view are handled by Command itself.
    function onKeydown(event: KeyboardEvent) {
        if (event.defaultPrevented) return;
        if (protocolError) {
            if (event.key === "Escape") {
                event.preventDefault();
                protocolError = false;
            }
            return;
        }
        if (panelOpen || panelPresent) return;

        if (event.key === "Escape") {
            event.preventDefault();
            failure = null;
            if (stack.length > 0) {
                // A command may still be working behind this view, and the user has
                // left it: stop it rather than paying for an answer nobody will read.
                invoke("cancel_invocation");
                abandonShownView();
                stack = stack.slice(0, -1);
                if (stack.length === 0) viewOwner = null;
            } else if (query.length > 0) {
                query = "";
            } else {
                invoke("dismiss");
            }
        } else if (stack.length > 0) {
            // A pushed view owns its own selection, so it owns its own action panel.
            return;
        } else if (
            event.key.toLowerCase() === "k" &&
            (event.ctrlKey || event.metaKey)
        ) {
            event.preventDefault();
            if (selectedItem && selectedItem.actions.length > 0)
                panelOpen = true;
        } else if (selectedItem) {
            for (const action of selectedItem.actions) {
                if (
                    action.shortcut &&
                    matchesShortcut(event, action.shortcut)
                ) {
                    event.preventDefault();
                    runAction(selectedItem, action.id);
                    break;
                }
            }
        }
    }

    onMount(() => {
        const updateVisibility = () => {
            document.documentElement.toggleAttribute("data-dango-hidden", document.hidden);
        };
        updateVisibility();
        document.addEventListener("visibilitychange", updateVisibility);
        // Runs while the window is still parked offscreen, so the first activation
        // already has the frecent items and their icons painted. Leaving it until
        // the first activation cost 40ms of icon loading and showed an empty list.
        runSearch("");
        invoke("warmup_done");
        const unlisten = [
            listen<number | null>("dango://activate", (event) => {
                inputEl?.focus();
                inputEl?.select();
                requestAnimationFrame(() =>
                    requestAnimationFrame(() => {
                        if (event.payload !== null)
                            invoke("report_paint", { id: event.payload });
                    }),
                );
            }),
            listen("dango://reset", () => resetToRoot()),
            listen<string>("dango://failed", (event) => {
                failure = event.payload;
                workingTitle = null;
            }),
            listen<ResultsPayload>("dango://results", (event) => {
                if (event.payload.query !== liveQuery) return;
                results = event.payload.items;
            }),
            listen<RenderPayload>("dango://render", (event) => {
                if (event.payload.invocation <= abandonedInvocation) return;
                lastInvocation = event.payload.invocation;
                workingTitle = null;
                if (event.payload.tree.protocolVersion !== PROTOCOL_VERSION) {
                    protocolError = true;
                    return;
                }
                viewOwner = event.payload.owner;
                if (
                    shownInvocation === event.payload.invocation &&
                    stack.length > 0
                ) {
                    stack = [...stack.slice(0, -1), event.payload.tree];
                } else {
                    stack = [...stack, event.payload.tree];
                }
                shownInvocation = event.payload.invocation;
            }),
        ];
        return () => {
            document.removeEventListener("visibilitychange", updateVisibility);
            document.documentElement.removeAttribute("data-dango-hidden");
            unlisten.forEach((p) => p.then((un) => un()));
        };
    });

    /// Who owns the cursor, in one place.
    ///
    /// The root input holds it unless something else is on screen that does: a
    /// pushed view focuses its own input, the action panel and the protocol error
    /// own the keyboard while they are up. Every one of those going away has to
    /// hand the cursor back, or the launcher is left unable to type, which is how
    /// popping a view used to strand it on the body.
    $effect(() => {
        if (stack.length > 0 || panelOpen || panelPresent || protocolError) return;
        inputEl?.focus();
    });

    // Re-query on every keystroke; the backend cancels the previous run.
    $effect(() => {
        const q = query;
        runSearch(q);
    });
</script>

<svelte:window onkeydown={onKeydown} onblur={() => invoke("dismiss")} />

<!-- The row is not focusable, so a click would otherwise move focus off the
     prompt and leave the launcher unable to type. The scroll margin keeps a
     group's heading in view when arrowing up lands on its first row. -->
{#snippet row(item: ResultItem)}
    <Command.Item
        value={item.id}
        onSelect={() => !panelOpen && !panelPresent && confirm(item)}
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

{#snippet group(heading: string, items: ResultItem[])}
    <Command.Group>
        <Command.GroupHeading

        >
            {heading}
        </Command.GroupHeading>
        <Command.GroupItems>
            {#each items as item (item.id)}
                {@render row(item)}
            {/each}
        </Command.GroupItems>
    </Command.Group>
{/snippet}

{#if protocolError}
    <LauncherFrame class="justify-center gap-2 px-6">
        <span class="text-foreground text-sm"
            >This view needs a newer version of Dango.</span
        >
        <span class="text-muted-foreground text-xs"
            >Press Escape to go back.</span
        >
    </LauncherFrame>
{:else if stack.length > 0}
    <LauncherFrame>
        <!-- Fills the space so the footer sits on the bottom edge rather than
         directly under a short form. -->
        <div class="flex min-h-0 flex-1 flex-col">
            <ProtocolView
                tree={stack[stack.length - 1]}
                onaction={runViewAction}
            >
                {#snippet beforeFooter()}
                    {#if failure}
                        <StatusBanner message={failure} />
                    {/if}
                {/snippet}
            </ProtocolView>
        </div>
    </LauncherFrame>
{:else}
    <Command.Root
        bind:value={
            () => selectedId, () => {}
        }
        onValueChange={(id) => { if (inUserGesture()) pickedId = id; }}
    >
        {#snippet child({ props })}
        <LauncherFrame {...props}>
        <Command.Input
            bind:ref={inputEl}
            bind:value={
                () => query,
                (q) => ((query = q), (pickedId = ""), (failure = null))
            }
            placeholder="Search apps and commands"

        />
        <!-- The inset lives outside the scroller, so the gap above the first row
         and below the last one is there at every scroll position instead of
         appearing only at the two ends. -->
        <div
            aria-busy={workingTitle !== null}
            class="border-border-card flex min-h-0 flex-1 flex-col border-t-edge py-2"
        >
            <Command.List
                data-pointer={pointerActive() ? "" : undefined}

            >
                <Command.Viewport >
                    {#if suggestions.length > 0}
                        {@render group("Suggestions", suggestions)}
                        {@render group("Everything else", everythingElse)}
                    {:else}
                        {#each results as item (item.id)}
                            {@render row(item)}
                        {/each}
                    {/if}
                    {#if results.length === 0 && query.length > 0}
                        <EmptyMessage class="truncate">No results for “{query}”</EmptyMessage>
                    {/if}
                </Command.Viewport>
            </Command.List>
        </div>

        {#if failure}
            <StatusBanner message={failure} />
        {/if}

        <LauncherFooter primaryLabel="Open" status={workingTitle ? `Running ${workingTitle}…` : ""}>
            {#snippet actions()}
                <ActionPanel
                    actions={selectedItem?.actions ?? []}
                    bind:open={panelOpen}
                    bind:present={panelPresent}
                    returnFocus={() => inputEl}
                    onrun={(id) => selectedItem && runAction(selectedItem, id)}
                />
            {/snippet}
        </LauncherFooter>
        </LauncherFrame>
        {/snippet}
    </Command.Root>
{/if}
