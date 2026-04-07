<script lang="ts" generics="T extends {id: string | number}">
    import { Button } from "$lib/components/ui/button";
    import * as ButtonGroup from "$lib/components/ui/button-group";
    import * as Select from "$lib/components/ui/select";
    import { X } from "@lucide/svelte";
    import type { Snippet } from "svelte";

    interface Props<T> {
        /** The options for this select. */
        options: T[];
        selected?: T;
        placeholder?: string;
        required?: boolean;

        clearable?: boolean;
        empty?: Snippet<[]>;
        trigger?: Snippet<[T]>;
        item?: Snippet<[T]>;
    }

    let {
        selected = $bindable(),
        options,
        placeholder,
        clearable,
        empty,
        item,
        trigger,
        ...selectProps
    }: Props<T> = $props();
</script>

{#snippet select()}
    <Select.Root
        type="single"
        bind:value={
            () => selected?.id?.toString(),
            (v) => (selected = options.find((o) => o.id == v))
        }
        {...selectProps}
    >
        <Select.Trigger class="w-full">
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
                <Select.Item value={opt.id.toString()}>
                    {#if item}
                        {@render item(opt)}
                    {:else}
                        {opt.id}
                    {/if}
                </Select.Item>
            {:else}
                {#if empty}
                    {@render empty()}
                {:else}
                    <em>No items</em>
                {/if}
            {/each}
        </Select.Content>
    </Select.Root>
{/snippet}

{#if clearable}
    <ButtonGroup.Root>
        {@render select()}

        <Button
            size="icon"
            variant="outline"
            onclick={() => (selected = undefined)}
        >
            <X />
        </Button>
    </ButtonGroup.Root>
{:else}
    {@render select()}
{/if}
