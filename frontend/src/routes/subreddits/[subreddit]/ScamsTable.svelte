<script lang="ts">
    import { Json } from "$lib/components/ui/json";
    import type {
        CreateOrUpdateScamInfo,
        CreateScamInfo,
        ScamInfo,
        UpdateScamInfo,
    } from "$lib/types/subreddit";
    import * as Table from "$lib/components/ui/table";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import * as Dialog from "$lib/components/ui/dialog";
    import ScamModalContent from "./ScamModalContent.svelte";
    import { ClipboardCopy, ClipboardPaste } from "@lucide/svelte";
    import * as Spinner from "$lib/components/ui/spinner";
    import { getSubredditScams } from "$lib/api/scams.remote";
    import { removeFirstInArrayBy, updateOrInsertInArrayBy } from "$lib/mutate";
    import TableChangesCell from "$lib/components/reuse/table/table-changes-cell.svelte";
    import { completeTransfer, initiateTransfer } from "$lib/types/transfer";
    import { toast } from "svelte-sonner";
    import ScamTransferModal from "$lib/components/scams/ScamTransferModal.svelte";
    import ScamInfoCell from "$lib/components/scams/ScamInfoCell.svelte";
    import TestScamModal from "$lib/components/scams/TestScamModal.svelte";

    interface ScamsTableProps {
        subreddit_id: string;

        removal_reasons: Record<string, string>;
        deleted_templates?: number[];

        updates: UpdateScamInfo[];
        creates: CreateScamInfo[];
        deletes: number[];
    }

    let modalItem = $state<CreateOrUpdateScamInfo | null>(null);
    let modalTransfer = $state<CreateScamInfo[] | null>(null);
    let modalTest = $state(false);

    let {
        subreddit_id,
        removal_reasons,
        deleted_templates,
        updates = $bindable(),
        creates = $bindable(),
        deletes = $bindable(),
    }: ScamsTableProps = $props();

    const fetchScams = $derived(getSubredditScams(subreddit_id));

    function mergeScamUpdates(
        scam: ScamInfo,
        updates: UpdateScamInfo[],
    ): [boolean, ScamInfo] {
        const update = updates.find((u) => u.id === scam.id);
        if (update) {
            return [
                true,
                {
                    ...scam,
                    ...update,
                },
            ];
        } else {
            return [false, scam];
        }
    }

    const deleteScam = (id: string | number) => {
        if (typeof id === "string") {
            removeFirstInArrayBy(creates, (s) => s.id === id);
        } else {
            deletes.push(id);
        }
    };

    const undeleteScam = (id: number) => {
        removeFirstInArrayBy(deletes, (v) => v === id);
    };

    const createScam = (scam: CreateScamInfo) => {
        creates.push(scam);
    };

    const updateScam = (update: UpdateScamInfo) => {
        updateOrInsertInArrayBy(updates, (s) => s.id === update.id, update);
        console.log("afterUpdate ", update.id, $state.snapshot(updates));
    };

    const onInitiateTransfer = () => {
        const items = (fetchScams.current ?? []).map(
            ({ template, ...item }) => {
                return item;
            },
        );

        const data = initiateTransfer(items);
        navigator.clipboard.writeText(data);

        toast.success(`Copied ${data.length} bytes`);
    };

    const onCompleteTransfer = async () => {
        try {
            const data = completeTransfer(await navigator.clipboard.readText());

            modalTransfer = data.map((item) => ({
                id: crypto.randomUUID(),
                ...item,
            }));
        } catch {
            toast.error("Failed to copy and/or parse data");
        }
    };
</script>

{#if modalItem}
    <Dialog.Root bind:open={() => true, (v) => (modalItem = null)}>
        <ScamModalContent
            subreddit={subreddit_id}
            {deleted_templates}
            {removal_reasons}
            bind:item={modalItem}
            onSubmit={(i) => {
                if (typeof i.id === "string") {
                    createScam(i as CreateScamInfo);
                } else {
                    updateScam(i as UpdateScamInfo);
                }
                modalItem = null;
            }}
        />
    </Dialog.Root>
{/if}

{#if modalTransfer}
    <Dialog.Root bind:open={() => true, (v) => (modalTransfer = null)}>
        {#if modalTransfer && fetchScams.current}
            <ScamTransferModal
                bind:transferred={modalTransfer}
                existing={fetchScams.current ?? []}
                {updateScam}
                {createScam}
                onClose={() => (modalTransfer = null)}
            />
        {/if}
    </Dialog.Root>
{/if}

{#if modalTest}
    <Dialog.Root bind:open={modalTest}>
        <TestScamModal onClose={() => (modalTest = false)} {subreddit_id} />
    </Dialog.Root>
{/if}

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head class="w-1"></Table.Head>
            <Table.Head>Name</Table.Head>
            <Table.Head>Info</Table.Head>
            <Table.Head class="w-1">Actions</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#if fetchScams.loading}
            <Table.Row>
                <Table.Cell colspan={4}>
                    <Spinner.Badge>Fetching rules</Spinner.Badge>
                </Table.Cell>
            </Table.Row>
        {/if}

        {#each fetchScams.current as original (original.id)}
            {@const deleted = deletes.indexOf(original.id) !== -1}
            {@const [updated, scam] = mergeScamUpdates(original, updates)}

            <Table.Row class={[deleted && "line-through"]}>
                <TableChangesCell {deleted} {updated} />
                <Table.Cell>
                    {scam.name}
                </Table.Cell>
                <ScamInfoCell {scam} />
                <Table.Cell>
                    {#if deleted}
                        <Button
                            variant="outline"
                            onclick={() => {
                                undeleteScam(scam.id);
                            }}>Restore</Button
                        >
                    {:else}
                        <Button
                            onclick={() => {
                                console.log("onEdit:", $state.snapshot(scam));
                                modalItem = scam;
                            }}>Edit</Button
                        >

                        <Button
                            variant="destructive"
                            onclick={() => {
                                deleteScam(scam.id);
                            }}>Delete</Button
                        >
                    {/if}
                </Table.Cell>
            </Table.Row>
        {/each}

        {#each creates as scam (scam.id)}
            <Table.Row>
                <TableChangesCell created />
                <Table.Cell>
                    {scam.name}
                </Table.Cell>
                <ScamInfoCell {scam} />
                <Table.Cell>
                    <Button
                        variant="destructive"
                        onclick={() => deleteScam(scam.id)}>Cancel</Button
                    >
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
    <Table.Footer>
        <Table.Row>
            <Table.Cell colspan={4}>
                <Button
                    size="icon-sm"
                    onclick={onInitiateTransfer}
                    title="Copy all scams"
                    ><ClipboardCopy />
                </Button>
                <Button
                    size="icon-sm"
                    onclick={onCompleteTransfer}
                    title="Paste all scams"><ClipboardPaste /></Button
                >

                <div class="float-end flex gap-1">
                    <Button size="sm" onclick={() => (modalTest = true)}
                        >Test</Button
                    >

                    <Button
                        size="sm"
                        onclick={() =>
                            (modalItem = {
                                id: crypto.randomUUID(),
                                enabled: true,
                                self_post: true,
                                name: "",
                                remove: false,
                                report: false,
                            })}>New</Button
                    >
                </div>
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>
