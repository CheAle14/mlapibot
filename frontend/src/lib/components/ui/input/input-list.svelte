<script lang="ts">
    import { Plus, Trash2, X } from "@lucide/svelte";
    import { Badge } from "../badge";
    import { Button, buttonVariants } from "../button";
    import * as Popover from "../popover";
    import Input from "./input.svelte";
    import { ScrollArea } from "../scroll-area";
    import { FormWrapped } from "$lib/components/reuse/form";
    import { useId } from "bits-ui";
    import { Separator } from "../separator";
    import type { ClassValue } from "clsx";
    import { cn } from "$lib/utils";

    interface StringProps {
        type: "text" | "url";
        value?: string[];

        noedit?: boolean;

        thingName?: string;

        popoverClass?: ClassValue;
    }

    type Props = StringProps;

    const formId = useId();
    let {
        type,
        thingName = "items",
        noedit = false,
        value = $bindable(),
        popoverClass,
    }: Props = $props();

    let pendingAddition = $state<any>();

    const onAdd = () => {
        if (value) {
            value.push(pendingAddition);
        } else {
            value = [pendingAddition];
        }
        pendingAddition = undefined;
    };

    const onRemove = (idx: number) => {
        if (value) value.splice(idx, 1);
    };
</script>

{#snippet trigger(items?: string[])}
    <Popover.Trigger class={buttonVariants({ variant: "outline" })}>
        {#if items && items.length > 0}
            Edit {items.length} {thingName}
        {:else}
            Add {thingName}
        {/if}
    </Popover.Trigger>
{/snippet}

<Popover.Root>
    {@render trigger(value)}

    <Popover.Content class={cn("flex flex-col", popoverClass)}>
        <FormWrapped id={formId} onsubmit={onAdd}>
            <div class="flex w-full gap-2">
                <Input required {type} bind:value={pendingAddition} />
                <Button type="submit" size="icon">
                    <Plus />
                </Button>
                <Button
                    type="button"
                    size="icon"
                    variant="destructive"
                    onclick={() => (value = undefined)}
                >
                    <Trash2 />
                </Button>
            </div>
        </FormWrapped>

        <Separator class="my-2" />

        <ScrollArea class="pr-3">
            <div class="min-h-24 max-h-72 flex flex-col gap-1">
                {#if value}
                    {#each value as item, i}
                        <div class="flex w-full gap-2">
                            <Input
                                disabled={noedit}
                                required
                                {type}
                                bind:value={value[i]}
                            />
                            <Button size="icon" onclick={() => onRemove(i)}>
                                <X />
                            </Button>
                        </div>
                    {/each}
                {/if}
            </div>
        </ScrollArea>
    </Popover.Content>
</Popover.Root>
