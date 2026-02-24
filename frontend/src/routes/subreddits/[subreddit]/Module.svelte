<script lang="ts" generics="T extends keyof SubredditOptions">
    import * as _ from "moderndash";
    import * as Accordion from "$lib/components/ui/accordion";
    import { Switch } from "$lib/components/ui/switch";
    import { Label } from "$lib/components/ui/label";
    import type {
        PendingSubredditOptions,
        SubredditOptions,
    } from "$lib/types/subreddit";
    import type { Snippet } from "svelte";
    import { mergeObjectPendingChanges } from "$lib/mutate.svelte";

    interface ModuleProps<T extends keyof SubredditOptions> {
        key: T;
        title: string;
        description?: string;

        children?: Snippet<
            [
                SubredditOptions[T],
                PendingSubredditOptions[T],
                SubredditOptions[T],
            ]
        >;

        options: SubredditOptions;
        changes: PendingSubredditOptions;
    }

    let {
        key,
        title,
        options,
        children,
        changes = $bindable(),
    }: ModuleProps<T> = $props();

    let moriginal = $derived(options[key]);
    let mpending = $derived(changes[key] as PendingSubredditOptions[T]);

    let mcurrent = $derived(
        mergeObjectPendingChanges(moriginal, mpending) as SubredditOptions[T],
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
            {@render children(mcurrent, mpending, moriginal)}
        {:else}
            There are no options for this module.
        {/if}
    </Accordion.Content>
</Accordion.Item>
