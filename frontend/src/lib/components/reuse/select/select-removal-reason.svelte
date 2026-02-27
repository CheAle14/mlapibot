<script lang="ts">
    import * as MySelect from "$lib/components/reuse/select";

    interface Props {
        reasons: Record<string, string>;
        value?: string;
    }

    let { reasons, value = $bindable() }: Props = $props();
    const options = $derived(
        Object.entries(reasons).map(([id, value]) => ({ id, value })),
    );
</script>

<MySelect.Simple
    {options}
    bind:selected={
        () => options.find((i) => i.id === value), (v) => (value = v?.id)
    }
>
    {#snippet trigger(v)}
        {v.id}
    {/snippet}

    {#snippet item(v)}
        <strong>{v.id}</strong>: {v.value}
    {/snippet}
</MySelect.Simple>
