<script lang="ts">
    import type {
        CreateScamInfo,
        ScamInfo,
        UpdateScamInfo,
    } from "$lib/types/subreddit";
    import * as Dialog from "../ui/dialog";
    import * as Table from "../ui/table";
    import { SvelteMap } from "svelte/reactivity";
    import SelectScam from "../reuse/select/select-scam.svelte";
    import { TableChangesCell } from "../reuse/table";
    import ScamInfoCell from "./ScamInfoCell.svelte";
    import { Button } from "../ui/button";
    import { isEqual } from "moderndash";
    import Json from "../ui/json/json.svelte";

    interface Props {
        transferred: CreateScamInfo[];
        existing: ScamInfo[];

        updateScam(info: UpdateScamInfo): void;
        createScam(info: CreateScamInfo): void;
        onClose(): void;
    }

    let {
        existing,
        updateScam,
        createScam,
        onClose,
        transferred = $bindable(),
    }: Props = $props();

    let { transfer_to_scam, scam_to_transfer } = $derived.by(() => {
        let transfer_to_scam = new SvelteMap<string, number>();
        let scam_to_transfer = new SvelteMap<number, string>();

        for (const transfer of transferred) {
            const withName = existing.find((s) => s.name === transfer.name);

            if (withName && !scam_to_transfer.has(withName.id)) {
                transfer_to_scam.set(transfer.id, withName.id);
                scam_to_transfer.set(withName.id, transfer.id);
            }
        }

        return { transfer_to_scam, scam_to_transfer };
    });

    const setMapping = (transfer: string, scam?: number) => {
        console.log("set mapping", transfer, scam);
        if (scam === undefined) {
            transfer_to_scam.delete(transfer);
        } else {
            // unlink any other transfer pointing to this scam
            const prior = scam_to_transfer.get(scam);
            if (prior && prior !== transfer) {
                transfer_to_scam.delete(prior);
            }

            // unlink the scam this transfer was pointing to
            let existing = transfer_to_scam.get(transfer);
            if (existing && existing !== scam) {
                scam_to_transfer.delete(existing);
            }

            transfer_to_scam.set(transfer, scam);
            scam_to_transfer.set(scam, transfer);
        }
    };

    const getMapping = (transfer: string): ScamInfo | undefined => {
        const map = transfer_to_scam.get(transfer);
        return map ? existing.find((s) => s.id === map) : undefined;
    };

    const onSubmit = () => {
        for (const transfer of transferred) {
            const mapping = getMapping(transfer.id);
            if (mapping) {
                const { id, ...update } = transfer;
                updateScam({
                    id: mapping.id,
                    ...update,
                });
            } else {
                createScam(transfer);
            }
        }

        onClose();
    };

    function toJson<K extends string | number, V>(
        map: SvelteMap<K, V>,
    ): Record<K, V> {
        const k = {} as Record<K, V>;
        for (const key of map.keys()) {
            k[key] = map.get(key) as V;
        }
        return k;
    }

    function getTranferState(transfer: CreateScamInfo, existing?: ScamInfo) {
        let created = false;
        let updated = false;
        let deleted = false;

        if (existing === undefined) {
            created = true;
        } else {
            const { id: _a, ...rest } = transfer;
            const { id: _b, ...rest2 } = existing;
            if (!isEqual(rest, rest2)) {
                updated = true;
            }
        }

        return { created, updated, deleted };
    }
</script>

<Dialog.Content size="large">
    <Dialog.Header>Paste multiple rules</Dialog.Header>

    <Json value={toJson(transfer_to_scam)} title="transfer -> scam" />
    <Json value={toJson(scam_to_transfer)} title="scam -> transfer" />

    <div>
        <Table.Root>
            <Table.Header>
                <Table.Row>
                    <Table.Head class="w-1"></Table.Head>
                    <Table.Head>Name</Table.Head>
                    <Table.Head>Incoming Info</Table.Head>
                    <Table.Head>Maps to</Table.Head>
                    <Table.Head>Mapped Info</Table.Head>
                </Table.Row>
            </Table.Header>
            <Table.Body>
                {#each transferred as transfer, i (transfer.id)}
                    {@const mapsTo = getMapping(transfer.id)}
                    {@const changes = getTranferState(transfer, mapsTo)}

                    <Table.Row data-map={mapsTo?.id}>
                        <TableChangesCell {...changes} />
                        <Table.Cell>
                            {transfer.name}
                        </Table.Cell>
                        <ScamInfoCell scam={transfer} />
                        <Table.Cell>
                            <SelectScam
                                scams={existing}
                                bind:selected={
                                    () => mapsTo,
                                    (v) => setMapping(transfer.id, v?.id)
                                }
                            />
                        </Table.Cell>
                        {#if mapsTo}
                            <ScamInfoCell scam={mapsTo} />
                        {:else}
                            <Table.Cell></Table.Cell>
                        {/if}
                    </Table.Row>
                {/each}
            </Table.Body>
        </Table.Root>
    </div>

    <Dialog.Footer>
        <Button type="submit" onclick={onSubmit}>Submit</Button>
    </Dialog.Footer>
</Dialog.Content>
