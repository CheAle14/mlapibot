<script lang="ts">
    import type { IMatcher } from "$lib/types/matcher";
    import { Input, InputClearable } from "../input";
    import * as Select from "../select";
    import { ScamMatcher } from ".";
    import * as ButtonGroup from "../button-group";
    import { Button } from "../button";

    interface Props {
        value: IMatcher | undefined;

        deleteSelf(): void;
    }

    let { value = $bindable(), deleteSelf }: Props = $props();

    const addNewChild = () => {
        if (value && value.type !== "phrase") {
            value.children.push({
                type: "phrase",
                phrase: "",
            });
        }
    };

    const onDeleteChild = (index: number) => {
        if (value && value.type !== "phrase") {
            value.children.splice(index, 1);
        }
    };
</script>

<div>
    <ButtonGroup.Root class="mb-0.5">
        {#if value && value.type !== "phrase"}
            <Button onclick={addNewChild}>+</Button>
        {/if}

        <Select.Root
            type="single"
            name="scamMatcher"
            bind:value={
                () => value?.type,
                (v) => {
                    if (v === "phrase") {
                        value = {
                            type: "phrase",
                            phrase: "",
                        };
                    } else if (v === "ordered" || v === "all" || v === "any") {
                        const children =
                            value && value.type !== "phrase"
                                ? value.children
                                : [];

                        value = {
                            type: v,
                            children,
                        };
                    } else if (v === undefined) {
                        value = undefined;
                    }
                }
            }
        >
            <Select.Trigger>
                {value?.type ?? "Select..."}
            </Select.Trigger>
            <Select.Content
                class="w-full min-w-24 grow"
                style={{ width: "100% !important" }}
            >
                <Select.Item value="phrase">Phrase</Select.Item>
                <Select.Item value="any">Any</Select.Item>
                <Select.Item value="ordered">Ordered</Select.Item>
                <Select.Item value="all">All</Select.Item>
            </Select.Content>
        </Select.Root>

        {#if value}
            {#if value.type === "phrase"}
                <Input type="text" bind:value={value.phrase} />
            {:else if value.type === "ordered"}
                <InputClearable
                    type="number"
                    bind:value={value.max_steps}
                    placeholder="Max steps"
                />
            {/if}
        {/if}

        <Button
            disabled={value === undefined}
            variant="destructive"
            onclick={deleteSelf}>X</Button
        >
    </ButtonGroup.Root>

    {#if value && value.type !== "phrase"}
        <div
            class="pl-2 pb-2 lg:pl-5 xl:pl-10 mb-2 border-l border-b border-primary border-solid"
        >
            {#each value.children as child, i}
                <ScamMatcher
                    bind:value={value.children[i]}
                    deleteSelf={() => onDeleteChild(i)}
                />
            {/each}
        </div>
    {/if}
</div>
