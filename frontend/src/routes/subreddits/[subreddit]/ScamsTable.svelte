<script lang="ts">
    import { Json } from "$lib/components/ui/json";
    import type { ScamInfo } from "$lib/types/subreddit";
    import * as Table from "$lib/components/ui/table";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import { toast } from "svelte-sonner";
    import * as Dialog from "$lib/components/ui/dialog";
    import ScamModalContent from "./ScamModalContent.svelte";
    import {
        isDeleted,
        stingifyDropDelete,
        SYM_DELETE,
    } from "$lib/types/deletable";
    import { Trash2 } from "@lucide/svelte";

    interface ScamsTableProps {
        scams: ScamInfo[];

        updateScam(id: number, scam: Partial<ScamInfo>): void;
    }

    let modalItem = $state<ScamInfo | null>(null);
    let { scams, updateScam }: ScamsTableProps = $props();

    let copyToClipboard = () => {
        const data = {
            v: 1,
            scams,
        };

        const text = btoa(stingifyDropDelete(data));
        navigator.clipboard.writeText(text);

        toast.success(`Copied ${text.length} bytes`);
    };

    let pasteFromClipboard = async () => {
        const b64 = await navigator.clipboard.readText();
        const text = atob(b64);
        const data = JSON.parse(text);

        if ("v" in data && data.v === 1) {
        } else {
            toast.error("Unrecognised paste data");
        }
    };
</script>

{#if modalItem}
    <Dialog.Root bind:open={() => true, (v) => (modalItem = null)}>
        <ScamModalContent
            bind:item={modalItem}
            onSubmit={(i) => {
                updateScam(i.id, i);
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
        {#each scams as scam (scam.id)}
            <Table.Row class={[scam[SYM_DELETE] && "line-through"]}>
                <Table.Cell class="flex flex-row">
                    {#if scam[SYM_DELETE]}
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

                        {#if scam.title}
                            <Badge>Title</Badge>
                        {/if}
                    </div>
                </Table.Cell>
                <Table.Cell>
                    {#if scam[SYM_DELETE]}
                        <Button
                            variant="outline"
                            onclick={() => {
                                updateScam(scam.id, {
                                    [SYM_DELETE]: undefined,
                                });
                            }}>Restore</Button
                        >
                    {:else}
                        <Button onclick={() => (modalItem = scam)}>Edit</Button>

                        <Button
                            variant="destructive"
                            onclick={() => {
                                updateScam(scam.id, {
                                    [SYM_DELETE]: true,
                                });
                            }}>Delete</Button
                        >
                    {/if}
                </Table.Cell>
            </Table.Row>
        {/each}
    </Table.Body>
    <Table.Footer>
        <Table.Row>
            <Table.Cell colspan={3}>
                <Button onclick={copyToClipboard}>Copy</Button>

                <Button class="float-end">New</Button>
            </Table.Cell>
        </Table.Row>
    </Table.Footer>
</Table.Root>

<Json value={scams} title="Scams" />
