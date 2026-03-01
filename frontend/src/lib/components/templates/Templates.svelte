<script lang="ts">
    import { Json } from "$lib/components/ui/json";
    import { Spinner } from "$lib/components/ui/spinner";
    import { getTemplateStubs } from "$lib/api/templates.remote";
    import * as Table from "$lib/components/ui/table";
    import { Button } from "$lib/components/ui/button";
    import { Dialog } from "bits-ui";
    import NewTemplateModal from "./NewTemplateModal.svelte";
    import {
        type CreateTemplateInfo,
        type EditTemplateInfo,
    } from "$lib/types/templates";
    import TemplateRow from "./TemplateRow.svelte";
    import type { SubredditTemplateStub } from "$lib/types/subreddit";
    import EditTemplateModal from "./EditTemplateModal.svelte";

    interface Props {
        subreddit: string;

        creates?: CreateTemplateInfo[];
        updates?: EditTemplateInfo[];
        deletes?: number[];
    }

    let {
        subreddit,
        creates = $bindable(),
        updates = $bindable(),
        deletes = $bindable(),
    }: Props = $props();

    const stubs = $derived(getTemplateStubs(subreddit));
    let modalNewItem = $state<CreateTemplateInfo | null>(null);
    let modalEditItem = $state<SubredditTemplateStub | EditTemplateInfo | null>(
        null,
    );

    const onAddNew = (v: CreateTemplateInfo) => {
        if (creates) {
            const idx = creates.findIndex((s) => s.id === v.id);
            if (idx !== -1) {
                creates[idx] = v;
            } else {
                creates.push(v);
            }
        } else {
            creates = [v];
        }

        modalNewItem = null;
    };

    const onEditExisting = (v: EditTemplateInfo) => {
        if (updates) {
            const idx = updates.findIndex((s) => s.id === v.id);
            if (idx !== -1) {
                updates[idx] = v;
            } else {
                updates.push(v);
            }
        } else {
            updates = [v];
        }

        modalEditItem = null;
    };

    const deleteNewItem = (id: string | number) => {
        if (typeof id === "string" && creates) {
            const idx = creates.findIndex((s) => s.id === id);
            creates.splice(idx, 1);
        } else if (typeof id === "number") {
            if (deletes) {
                deletes.push(id);
            } else {
                deletes = [id];
            }
        }
    };

    $inspect(updates);
</script>

<Dialog.Root
    bind:open={() => modalNewItem !== null, () => (modalNewItem = null)}
>
    {#if modalNewItem}
        <NewTemplateModal {...modalNewItem} onSubmit={onAddNew} />
    {/if}
</Dialog.Root>

<Dialog.Root
    bind:open={() => modalEditItem !== null, () => (modalEditItem = null)}
>
    {#if modalEditItem}
        <EditTemplateModal
            {subreddit}
            {...modalEditItem}
            onSubmit={onEditExisting}
        />
    {/if}
</Dialog.Root>

{#if stubs.loading}
    <Spinner />
{:else}
    <Table.Root>
        <Table.Header>
            <Table.Row>
                <Table.Head class="w-1"></Table.Head>
                <Table.Head>Name</Table.Head>
                <Table.Head class="w-1">Actions</Table.Head>
            </Table.Row>
        </Table.Header>

        <Table.Body>
            {#each stubs.current as stub}
                {@const changes = updates?.find((s) => s.id === stub.id)}

                <TemplateRow
                    item={stub}
                    {changes}
                    bind:deleted={
                        () => !!deletes && deletes.indexOf(stub.id) !== -1,
                        (v) => {
                            if (v) {
                                if (deletes) {
                                    deletes.push(stub.id);
                                } else {
                                    deletes = [stub.id];
                                }
                            } else if (deletes) {
                                deletes = deletes.filter((s) => s !== stub.id);
                            }
                        }
                    }
                    openModal={() => (modalEditItem = changes ?? stub)}
                />
            {/each}

            {#each creates as created}
                <TemplateRow
                    item={created}
                    bind:deleted={() => false, () => deleteNewItem(created.id)}
                    openModal={() => (modalNewItem = created)}
                />
            {/each}
        </Table.Body>
        <Table.Footer>
            <Table.Row>
                <Table.Cell colspan={3}>
                    <Button
                        class="float-end"
                        size="sm"
                        onclick={() =>
                            (modalNewItem = {
                                id: crypto.randomUUID(),
                                name: "",
                                content: "",
                            })}>New</Button
                    >
                </Table.Cell>
            </Table.Row>
        </Table.Footer>
    </Table.Root>

    <ul></ul>
{/if}
