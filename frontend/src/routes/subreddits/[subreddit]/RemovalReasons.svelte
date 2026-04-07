<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Table from "$lib/components/ui/table";
    import { Trash2 } from "@lucide/svelte";
    import type { RemReasonItem } from "./RemReasonModal.svelte";
    import RemReasonModal from "./RemReasonModal.svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import TableChangesCell from "$lib/components/reuse/table/table-changes-cell.svelte";

    interface Props {
        original: Record<string, string>;
        current: Record<string, string>;
    }

    let { original, current = $bindable() }: Props = $props();

    const deleteItem = (key: string) => {
        delete current[key];
        console.log("deleted", key);
    };

    const undeleteItem = (key: string) => {
        current[key] = original[key];
    };

    let modalItem = $state<RemReasonItem | null>(null);

    const allKeys: Record<string, string> = $derived({
        ...original,
        ...current,
    });
</script>

{#if modalItem}
    <Dialog.Root bind:open={() => true, (v) => (modalItem = null)}>
        <RemReasonModal
            bind:item={modalItem}
            onSubmit={(i) => {
                current[i.key] = i.value;
                modalItem = null;
            }}
        />
    </Dialog.Root>
{/if}

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head class="w-1"></Table.Head>
            <Table.Head>Alias</Table.Head>
            <Table.Head>Reason UUID</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#each Object.entries(allKeys) as [alias, reason_id]}
            {@const created = original[alias] === undefined}
            {@const updated = original[alias] !== current[alias]}
            {@const deleted = current[alias] === undefined}

            <Table.Row class={[deleted && "line-through"]}>
                <TableChangesCell {deleted} {updated} {created} />
                <Table.Cell>
                    {alias}
                </Table.Cell>
                <Table.Cell>
                    {reason_id}
                </Table.Cell>
                <Table.Cell class="flex justify-end  gap-2">
                    {#if deleted}
                        <Button
                            class="float-end"
                            variant="outline"
                            onclick={() => undeleteItem(alias)}>Restore</Button
                        >
                    {:else}
                        <Button
                            class="float-end"
                            onclick={() =>
                                (modalItem = {
                                    adding: false,
                                    key: alias,
                                    value: reason_id,
                                })}>Edit</Button
                        >

                        <Button
                            class="float-end"
                            variant="destructive"
                            onclick={() => deleteItem(alias)}>Delete</Button
                        >
                    {/if}
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
    <Table.Footer>
        <Table.Row>
            <Table.Cell colspan={4}>
                <Button
                    size="sm"
                    class="float-end"
                    onclick={() =>
                        (modalItem = { adding: true, key: "", value: "" })}
                    >New</Button
                >
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>
