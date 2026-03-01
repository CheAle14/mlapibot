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
    import { Pencil, Plus, Trash2 } from "@lucide/svelte";
    import * as Spinner from "$lib/components/ui/spinner";
    import { createQuery } from "@tanstack/svelte-query";
    import { fetchSubredditScams } from "$lib/queries/subreddits";

    interface ScamsTableProps {
        subreddit_id: string;

        removal_reasons: Record<string, string>;
        deleted_templates?: number[];

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
        removal_reasons,
        deleted_templates,
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
        {#if fetchScams.isFetching}
            <Table.Row>
                <Table.Cell colspan={4}>
                    <Spinner.Badge>Fetching rules</Spinner.Badge>
                </Table.Cell>
            </Table.Row>
        {/if}

        {#each fetchScams.data as original (original.id)}
            {@const isDeleted =
                original.id && deletes.indexOf(original.id) !== -1}
            {@const [isUpdated, scam] = mergeScamUpdates(original, updates)}

            <Table.Row class={[isDeleted && "line-through"]}>
                <Table.Cell>
                    {#if isDeleted}
                        <Trash2 />
                    {:else if isUpdated}
                        <Pencil />
                    {/if}
                </Table.Cell>
                <Table.Cell class="flex flex-row">
                    {scam.name}
                </Table.Cell>
                <Table.Cell>
                    <div class="float-start">
                        {#if !scam.enabled}
                            <Badge variant="outline">Disabled</Badge>
                        {/if}

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
                <Table.Cell>
                    <Plus />
                </Table.Cell>
                <Table.Cell>
                    {scam.name}
                </Table.Cell>
                <Table.Cell>
                    <div class="float-start">
                        {#if !scam.enabled}
                            <Badge variant="outline">Disabled</Badge>
                        {/if}

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
                            enabled: true,
                            self_post: true,
                            name: "",
                            remove: false,
                            report: false,
                        })}>New</Button
                >
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>
