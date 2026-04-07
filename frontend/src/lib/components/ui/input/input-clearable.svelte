<script lang="ts">
    import * as InputGroup from "$lib/components/ui/input-group";
    import { X } from "@lucide/svelte";

    type VariousProps = {
        id?: string;
        type: "text" | "number";
        placeholder?: string;
        value: string | number | undefined;
        coerceEmpty?: boolean;
        required?: boolean;
        onClear?: () => void;
    };

    let {
        value = $bindable(),
        coerceEmpty = false,
        onClear,
        ...rest
    }: VariousProps = $props();

    let coercedValue = $derived.by(() => {
        if (coerceEmpty && value === "") return undefined;
        if (value === null) return undefined;
        return value;
    });
</script>

<InputGroup.Root>
    <InputGroup.Input
        bind:value={() => coercedValue, (v) => (value = v)}
        {...rest}
    />

    {#if coercedValue !== undefined}
        <InputGroup.Addon align="inline-end">
            <InputGroup.Button
                size="icon-xs"
                onclick={onClear ?? (() => (value = undefined))}
            >
                <X />
            </InputGroup.Button>
        </InputGroup.Addon>
    {/if}
</InputGroup.Root>
