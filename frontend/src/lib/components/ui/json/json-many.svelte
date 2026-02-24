<script lang="ts">
    import Json from "./json.svelte";

    interface JsonManyProps {
        items: unknown[];
        titles?: string[];
    }

    let { items, titles }: JsonManyProps = $props();
</script>

<div class="flex flex-row w-full justify-between">
    {#each items as item, i}
        <Json value={item} title={titles?.at(i)} />
    {/each}
</div>
