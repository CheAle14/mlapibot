<script lang="ts">
    import * as MySelect from "$lib/components/reuse/select";
    import * as Select from "$lib/components/ui/select";
    import type { StatusIncidentImpact } from "$lib/types/subreddit";

    interface Props {
        value?: StatusIncidentImpact;
    }

    interface Opt {
        id: StatusIncidentImpact;
        name: string;
    }

    const OPTIONS: Opt[] = [
        { id: "none", name: "None" },
        { id: "maintenance", name: "Maintenance" },
        { id: "minor", name: "Minor" },
        { id: "major", name: "Major" },
        { id: "critical", name: "Critical" },
    ];

    let { value = $bindable() }: Props = $props();
</script>

<MySelect.Simple
    options={OPTIONS}
    bind:selected={
        () => OPTIONS.find((i) => i.id === value), (v) => (value = v?.id)
    }
>
    {#snippet item(v)}
        {v.name}
    {/snippet}
</MySelect.Simple>
