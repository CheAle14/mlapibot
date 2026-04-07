<script lang="ts">
    import type { ScamInfo } from "$lib/types/subreddit";
    import Select from "./select-simple.svelte";

    interface Props {
        scams: ScamInfo[];
        selected?: ScamInfo;
    }

    let { scams, selected = $bindable() }: Props = $props();
</script>

<Select
    options={scams}
    bind:selected
    clearable
    placeholder="Creating new, or select..."
>
    {#snippet item(item)}
        {item.id}: {item.name}
    {/snippet}
</Select>
