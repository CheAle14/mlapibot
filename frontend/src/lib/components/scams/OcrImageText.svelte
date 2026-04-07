<script lang="ts">
    import { SvelteSet } from "svelte/reactivity";

    interface Props {
        text: string;
        triggers: number[];
    }

    let { text, triggers: triggerList }: Props = $props();
    let triggers = $derived(new SvelteSet(triggerList));
</script>

<p class="whitespace-pre-wrap">
    {#each text.split(" ") as word, idx}
        {#if triggers.has(idx)}
            <strong class="text-destructive">{word + " "}</strong>
        {:else}
            {word + " "}
        {/if}
    {/each}
</p>
