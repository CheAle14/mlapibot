<script lang="ts" generics="T extends keyof ApiSubredditModules">
    import * as _ from "moderndash";
    import * as Accordion from "$lib/components/ui/accordion";
    import { Switch } from "$lib/components/ui/switch";
    import { Label } from "$lib/components/ui/label";
    import type {
        ApiSubredditModules,
        ApiSubredditOptions,
    } from "$lib/types/subreddit";
    import type { Snippet } from "svelte";

    interface SnippetArgs<T extends keyof ApiSubredditModules> {
        current: ApiSubredditOptions[T];
        original: ApiSubredditOptions[T];
        open: boolean;
    }

    interface ModuleProps<T extends keyof ApiSubredditModules> {
        key: T;
        title: string;
        description?: string;
        open: string[];

        children?: Snippet<[SnippetArgs<T>]>;

        original: ApiSubredditOptions;
        current: ApiSubredditOptions;
    }

    let {
        key,
        title,
        open,
        children,
        original,
        current = $bindable(),
    }: ModuleProps<T> = $props();

    let moriginal = $derived(original[key]);
    let mcurrent = $derived(current[key]);
</script>

<Accordion.Item value={key}>
    <Accordion.Trigger>
        {#snippet outside()}
            <Switch
                id={key}
                checked={mcurrent.enabled}
                onclick={(e) => {
                    mcurrent.enabled = !mcurrent.enabled;
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
                original: moriginal,
                open: open?.some((s) => s === key),
            })}
        {:else}
            There are no options for this module.
        {/if}
    </Accordion.Content>
</Accordion.Item>
