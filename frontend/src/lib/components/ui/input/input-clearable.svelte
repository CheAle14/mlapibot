<script lang="ts">
    import * as InputGroup from "$lib/components/ui/input-group";
    import { X } from "@lucide/svelte";

    type VariousProps = {
        type: "text" | "number";
        placeholder?: string;
        value: string | number | undefined;
    };

    let { value = $bindable(), ...rest } = $props();
</script>

<InputGroup.Root>
    <InputGroup.Input bind:value {...rest} />

    {#if value !== undefined}
        <InputGroup.Addon align="inline-end">
            <InputGroup.Button
                size="icon-xs"
                onclick={() => (value = undefined)}
            >
                <X />
            </InputGroup.Button>
        </InputGroup.Addon>
    {/if}
</InputGroup.Root>
