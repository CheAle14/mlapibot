<script lang="ts" generics="T extends keyof PendingSubredditModules">
    import * as _ from "moderndash";
    import * as Accordion from "$lib/components/ui/accordion";
    import { Switch } from "$lib/components/ui/switch";
    import { Label } from "$lib/components/ui/label";
    import type {
        PendingSubredditModules,
        PendingSubredditOptions,
        SubredditOptions,
    } from "$lib/types/subreddit";
    import type { Snippet } from "svelte";
    import { mergeObjectPendingChanges } from "$lib/mutate.svelte";

    interface SnippetArgs<T extends keyof PendingSubredditModules> {
        current: SubredditOptions[T];
        pending: PendingSubredditOptions[T];
        original: SubredditOptions[T];
        open: boolean;
    }

    interface ModuleProps<T extends keyof PendingSubredditModules> {
        key: T;
        title: string;
        description?: string;
        open: string[];

        children?: Snippet<[SnippetArgs<T>]>;

        options: SubredditOptions;
        changes: PendingSubredditOptions;
    }

    let {
        key,
        title,
        open,
        options,
        children,
        changes = $bindable(),
    }: ModuleProps<T> = $props();

    let moriginal = $derived(options[key]);
    let mpending = $derived(changes[key] as PendingSubredditModules[T]);

    if (key === "scams") {
        $inspect(moriginal, mpending);
    }

    let mcurrent = $derived(
        mergeObjectPendingChanges(
            moriginal,
            mpending as any,
        ) as SubredditOptions[T],
    );
</script>

<Accordion.Item value={key}>
    <Accordion.Trigger>
        {#snippet outside()}
            <Switch
                id={key}
                checked={mcurrent.enabled}
                onclick={(e) => {
                    mpending.enabled = !mcurrent.enabled;
                    e.preventDefault();
                    e.stopPropagation();
                    return false;
                }}
            />
        {/snippet}
        <Label>{title}</Label>
    </Accordion.Trigger>

    <Accordion.Content class="pl-2">
        {#if children}
            {@render children({
                current: mcurrent,
                pending: mpending,
                original: moriginal,
                open: open?.some((s) => s === key),
            })}
        {:else}
            There are no options for this module.
        {/if}
    </Accordion.Content>
</Accordion.Item>
