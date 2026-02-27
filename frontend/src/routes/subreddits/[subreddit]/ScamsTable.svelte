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
    import { Plus, Trash2 } from "@lucide/svelte";
    import * as Spinner from "$lib/components/ui/spinner";
    import { createQuery } from "@tanstack/svelte-query";
    import { fetchSubredditScams } from "$lib/queries/subreddits";

    interface ScamsTableProps {
        subreddit_id: string;

        updates: UpdateScamInfo[];
        creates: CreateScamInfo[];
        deletes: number[];

        createScam(scam: CreateScamInfo): void;
        updateScam(scam: UpdateScamInfo): void;
        deleteScam(id: number): void;

        uncreateScam(id: string): void;
        undeleteScam(id: number): void;
    }

    let modalItem = $state<CreateOrUpdateScamInfo | null>(null);
    let {
        subreddit_id,
        updates,
        creates,
        deletes,
        createScam,
        updateScam,
        deleteScam,
        uncreateScam,
        undeleteScam,
    }: ScamsTableProps = $props();

    const fetchScams = createQuery(() => ({
        queryKey: ["subreddits", subreddit_id, "scams"],
        queryFn: () => fetchSubredditScams(subreddit_id),
    }));

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
</script>

{#if modalItem}
    <Dialog.Root bind:open={() => true, (v) => (modalItem = null)}>
        <ScamModalContent
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

<Table.Root>
    <Table.Header>
        <Table.Row>
            <Table.Head>Name</Table.Head>
            <Table.Head>Info</Table.Head>
            <Table.Head>Actions</Table.Head>
        </Table.Row>
    </Table.Header>
    <Table.Body>
        {#if fetchScams.isFetching}
            <Table.Row>
                <Table.Cell colspan={3}>
                    <Spinner.Badge>Fetching rules</Spinner.Badge>
                </Table.Cell>
            </Table.Row>
        {/if}

        {#each fetchScams.data as original (original.id)}
            {@const isDeleted =
                original.id && deletes.indexOf(original.id) !== -1}
            {@const [isUpdated, scam] = mergeScamUpdates(original, updates)}

            <Table.Row
                class={[isDeleted && "line-through"]}
                data-id={original.id}
                data-deleted={isDeleted}
                data-deletes={JSON.stringify(deletes)}
            >
                <Table.Cell class="flex flex-row">
                    {#if isDeleted}
                        <Trash2 />
                    {/if}
                    {scam.name}
                </Table.Cell>
                <Table.Cell>
                    <div class="float-start">
                        {#if scam.report && scam.remove}
                            <Badge variant="secondary">Filter</Badge>
                        {:else if scam.report}
                            <Badge>Report</Badge>
                        {:else if scam.remove}
                            <Badge variant="destructive">Remove</Badge>
                        {:else}
                            <!-- no action -->
                        {/if}
                    </div>

                    <div class="float-end">
                        {#if scam.ocr}
                            <Badge>OCR</Badge>
                        {/if}

                        {#if scam.title ?? scam.title_or_body}
                            <Badge>Title</Badge>
                        {/if}

                        {#if scam.body ?? scam.title_or_body}
                            <Badge>Body</Badge>
                        {/if}
                    </div>
                </Table.Cell>
                <Table.Cell>
                    {#if isDeleted}
                        <Button
                            variant="outline"
                            onclick={() => {
                                undeleteScam(scam.id);
                            }}>Restore</Button
                        >
                    {:else}
                        <Button onclick={() => (modalItem = scam)}>Edit</Button>

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
                <Table.Cell class="flex flex-row">
                    <Plus />
                    {scam.name}
                </Table.Cell>
                <Table.Cell>
                    <div class="float-start">
                        {#if scam.report && scam.remove}
                            <Badge variant="secondary">Filter</Badge>
                        {:else if scam.report}
                            <Badge>Report</Badge>
                        {:else if scam.remove}
                            <Badge variant="destructive">Remove</Badge>
                        {:else}
                            <!-- no action -->
                        {/if}
                    </div>

                    <div class="float-end">
                        {#if scam.ocr}
                            <Badge>OCR</Badge>
                        {/if}

                        {#if scam.title ?? scam.title_or_body}
                            <Badge>Title</Badge>
                        {/if}

                        {#if scam.body ?? scam.title_or_body}
                            <Badge>Body</Badge>
                        {/if}
                    </div>
                </Table.Cell>
                <Table.Cell>
                    <Button
                        variant="destructive"
                        onclick={() => uncreateScam(scam.id)}>Cancel</Button
                    >
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
                        (modalItem = {
                            id: crypto.randomUUID(),
                            name: "",
                            template: "default",
                            remove: false,
                            report: false,
                        })}>New</Button
                >
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>
