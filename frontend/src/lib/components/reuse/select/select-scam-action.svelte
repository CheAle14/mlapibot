<script lang="ts">
    import * as MySelect from "$lib/components/reuse/select";
    import { Json } from "$lib/components/ui/json";
    import * as Select from "$lib/components/ui/select";
    import SelectGroupHeading from "$lib/components/ui/select/select-group-heading.svelte";
    import type { CreateOrUpdateScamInfo } from "$lib/types/subreddit";

    interface Props {
        scam?: CreateOrUpdateScamInfo;
    }

    interface Opt {
        id: string;
        description?: string;

        report: boolean;
        remove: boolean;
    }

    const OPTIONS: Opt[] = [
        {
            id: "None",
            report: false,
            remove: false,
        },
        {
            id: "Report",
            report: true,
            remove: false,
        },
        {
            id: "Remove",
            report: false,
            remove: true,
        },
        {
            id: "Filter",
            description: "Removes the post and sends a modmail",
            report: true,
            remove: true,
        },
    ];

    function compBool(v0: boolean, v1: boolean | undefined) {
        return v0 === !!v1;
    }

    let { scam = $bindable() }: Props = $props();
</script>

<MySelect.Simple
    options={OPTIONS}
    bind:selected={
        () =>
            OPTIONS.find(
                (o) =>
                    compBool(o.report, scam?.report) &&
                    compBool(o.remove, scam?.remove),
            ),
        (v) => {
            if (scam && v) {
                scam.report = v.report;
                scam.remove = v.remove;
            }
        }
    }
>
    {#snippet trigger(v)}
        {v.id}
    {/snippet}

    {#snippet item(v)}
        <div class="flex flex-col">
            <strong>{v.id}</strong>

            {#if v.description}
                <p>{v.description}</p>
            {/if}
        </div>
    {/snippet}
</MySelect.Simple>
