<script lang="ts">
    import { TableChangesCell } from "$lib/components/reuse/table";
    import { Button, buttonVariants } from "$lib/components/ui/button";
    import { Dialog } from "$lib/components/ui/dialog";
    import Json from "$lib/components/ui/json/json.svelte";
    import * as Table from "$lib/components/ui/table";
    import { removeAtIndex, updateOrInsertInArrayBy } from "$lib/mutate";
    import type {
        ComplexCommentInfo,
        ComplexCommentModalItem,
    } from "$lib/types/subreddit";
    import ComplexCommentModal from "./ComplexCommentModal.svelte";

    interface Props {
        removal_reasons: Record<string, string>;
        original?: ComplexCommentInfo[];
        current?: ComplexCommentInfo[];
    }

    let { removal_reasons, original, current = $bindable() }: Props = $props();

    let modalItem = $state<ComplexCommentModalItem | null>(null);

    const startEditItem = (item: ComplexCommentInfo) => {
        const { name, link_title, comment, reason, ignore_flairs } = item;

        modalItem = {
            name,
            old_name: name,
            reason,
            link_title: [...link_title],
            comment: [...comment],
            ignore_flairs: ignore_flairs ? [...ignore_flairs] : undefined,
        };
    };
</script>

<Dialog bind:open={() => true, (v) => (modalItem = null)}>
    {#if modalItem}
        <ComplexCommentModal
            {removal_reasons}
            bind:item={modalItem}
            onSubmit={(i) => {
                if (current) {
                    updateOrInsertInArrayBy(
                        current,
                        (s) => s.name === (i.old_name ?? i.name),
                        i,
                    );
                } else {
                    current = [i];
                }
                modalItem = null;
            }}
        />
    {/if}
</Dialog>

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head class="w-1"></Table.Head>
            <Table.Head>Name</Table.Head>
            <Table.Head class="w-1">Actions</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#each current as item, i}
            {@const updated =
                !original || !original.some((s) => s.name === item.name)}

            <Table.Row>
                <TableChangesCell {updated} />
                <Table.Cell>
                    {item.name}
                </Table.Cell>
                <Table.Cell>
                    <Button onclick={() => startEditItem(item)}>Edit</Button>
                    <Button
                        variant="destructive"
                        onclick={() => removeAtIndex(current, i)}>Delete</Button
                    >
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
    <Table.Footer>
        <Table.Row>
            <Table.Cell colspan={3}>
                <Button
                    class="float-right"
                    size="sm"
                    onclick={() =>
                        (modalItem = {
                            name: "",
                            link_title: [],
                            comment: [],
                            reason: "",
                        })}>New</Button
                >
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>
