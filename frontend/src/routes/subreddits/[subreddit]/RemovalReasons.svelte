<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Table from "$lib/components/ui/table";
    import { Trash2 } from "@lucide/svelte";
    import type { RemReasonItem } from "./RemReasonModal.svelte";
    import RemReasonModal from "./RemReasonModal.svelte";
    import * as Dialog from "$lib/components/ui/dialog";

    interface Props {
        reasons: Record<string, string>;
        updates?: Record<string, string>;
        deletes?: string[];
    }

    let {
        reasons,
        updates = $bindable(),
        deletes = $bindable(),
    }: Props = $props();

    const deleteItem = (key: string) => {
        if (deletes) {
            deletes.push(key);
        } else {
            deletes = [key];
        }

        console.log("deleted", key);
    };

    const undeleteItem = (key: string) => {
        if (deletes) {
            deletes = deletes.filter((s) => s !== key);
        }
    };

    let modalItem = $state<RemReasonItem | null>(null);

    let current = $derived({ ...reasons, ...(updates ?? {}) });
</script>

{#if modalItem}
    <Dialog.Root bind:open={() => true, (v) => (modalItem = null)}>
        <RemReasonModal
            bind:item={modalItem}
            onSubmit={(i) => {
                if (updates) {
                    updates[i.key] = i.value;
                } else {
                    updates = { [i.key]: i.value };
                }
                modalItem = null;
            }}
        />
    </Dialog.Root>
{/if}

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head>Alias</Table.Head>
            <Table.Head>Reason UUID</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#each Object.entries(current) as [alias, reason_id]}
            {@const isDeleted = deletes && deletes.indexOf(alias) !== -1}

            <Table.Row class={[isDeleted && "line-through"]}>
                <Table.Cell>
                    {#if isDeleted}
                        <Trash2 />
                    {/if}

                    {alias}
                </Table.Cell>
                <Table.Cell>
                    {reason_id}
                </Table.Cell>
                <Table.Cell class="flex justify-end  gap-2">
                    {#if isDeleted}
                        <Button
                            class="float-end"
                            variant="outline"
                            onclick={() => undeleteItem(alias)}>Restore</Button
                        >
                    {:else}
                        <Button
                            class="float-end"
                            variant="destructive"
                            onclick={() => deleteItem(alias)}>Delete</Button
                        >
                        <Button
                            class="float-end"
                            onclick={() =>
                                (modalItem = {
                                    adding: false,
                                    key: alias,
                                    value: reason_id,
                                })}>Edit</Button
                        >
                    {/if}
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
    <Table.Footer>
        <Table.Row>
            <Table.Cell colspan={3}>
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
