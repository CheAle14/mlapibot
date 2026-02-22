<script lang="ts">
    import * as Item from "$lib/components/ui/item/index.js";

    interface JsonProps {
        title?: string;
        spaces?: number | string;
        value: unknown;
    }

    let { title, value, spaces }: JsonProps = $props();
</script>

<Item.Root variant="outline" class="w-3/4">
    <Item.Content>
        {#if title}
            <Item.Title>{title}</Item.Title>
        {/if}

        <pre>{JSON.stringify(value, undefined, spaces ?? 4)}</pre>
    </Item.Content>
</Item.Root>
