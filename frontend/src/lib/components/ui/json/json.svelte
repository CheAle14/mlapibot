<script lang="ts">
    import * as Item from "$lib/components/ui/item/index.js";
    import { isHttpError } from "@sveltejs/kit";

    interface JsonProps {
        title?: string;
        spaces?: number | string;
        value: unknown;
    }

    let { title, value, spaces }: JsonProps = $props();
</script>

<Item.Root variant="outline" class="w-auto m-2">
    {#if title}
        <Item.Header>{title}</Item.Header>
    {/if}
    <Item.Content>
        {#if isHttpError(value)}
            <Item.Title>{value.status}</Item.Title>
            <pre>{JSON.stringify(value.body, undefined, spaces ?? 4)}</pre>
        {:else}
            <pre>{JSON.stringify(value, undefined, spaces ?? 4)}</pre>
        {/if}
    </Item.Content>
</Item.Root>
