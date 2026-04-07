<script lang="ts">
    import TableCell from "$lib/components/ui/table/table-cell.svelte";
    import { Pencil, Plus, Trash2 } from "@lucide/svelte";

    interface Props {
        created?: boolean;
        updated?: boolean;
        deleted?: boolean;
    }

    let { created, updated, deleted }: Props = $props();
</script>

<TableCell>
    {#if deleted}
        <Trash2 />
    {:else if created}
        <Plus />
    {:else if updated}
        <Pencil />
    {/if}
</TableCell>
