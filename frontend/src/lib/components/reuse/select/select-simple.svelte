<script lang="ts" generics="T extends {id: string}">
    import { Json } from "$lib/components/ui/json";
    import * as Select from "$lib/components/ui/select";
    import type { StatusIncidentImpact } from "$lib/types/subreddit";
    import type { Snippet } from "svelte";

    interface Props<T> {
        /** The options for this select. */
        options: T[];
        selected?: T;
        placeholder?: string;

        trigger?: Snippet<[T]>;
        item?: Snippet<[T]>;
    }

    let {
        selected = $bindable(),
        options,
        placeholder,
        item,
        trigger,
    }: Props<T> = $props();
</script>

<Select.Root
    type="single"
    bind:value={
        () => selected?.id, (v) => (selected = options.find((o) => o.id === v))
    }
>
    <Select.Trigger>
        {#if selected}
            {#if trigger}
                {@render trigger(selected)}
            {:else if item}
                {@render item(selected)}
            {:else}
                {selected.id}
            {/if}
        {:else}
            {placeholder ?? "Select item.."}
        {/if}
    </Select.Trigger>
    <Select.Content>
        {#each options as opt}
            <Select.Item value={opt.id}>
                {#if item}
                    {@render item(opt)}
                {:else}
                    {opt.id}
                {/if}
            </Select.Item>
        {/each}
    </Select.Content>
</Select.Root>
