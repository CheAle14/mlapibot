<script lang="ts">
    import {
        isCreatingTemplate,
        type CreateOrStubTemplateInfo,
        type EditTemplateInfo,
    } from "$lib/types/templates";

    import { Row, Cell } from "$lib/components/ui/table";
    import { Button } from "$lib/components/ui/button";
    import { Pencil, Plus, Trash2 } from "@lucide/svelte";

    interface Props {
        item: CreateOrStubTemplateInfo;

        changes?: EditTemplateInfo;

        deleted: boolean;
        openModal(): void;
    }

    let { item, changes, deleted = $bindable(), openModal }: Props = $props();

    $inspect(item, changes);
</script>

<Row>
    <Cell>
        {#if isCreatingTemplate(item)}
            <Plus />
        {:else if deleted}
            <Trash2 />
        {:else if changes}
            <Pencil />
        {/if}
    </Cell>
    <Cell>
        {changes?.name ?? item.name}
    </Cell>
    <Cell>
        <Button onclick={openModal}>Edit</Button>

        <Button
            onclick={() => (deleted = !deleted)}
            variant={deleted ? "ghost" : "destructive"}
        >
            {#if deleted}
                Restore
            {:else if isCreatingTemplate(item)}
                Cancel
            {:else}
                Delete
            {/if}
        </Button>
    </Cell>
</Row>
