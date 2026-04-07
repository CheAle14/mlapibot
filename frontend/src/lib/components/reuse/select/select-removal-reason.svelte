<script lang="ts">
    import * as MySelect from "$lib/components/reuse/select";

    interface Props {
        required?: boolean;
        reasons: Record<string, string>;
        value?: string;
    }

    let { required, reasons, value = $bindable() }: Props = $props();
    const options = $derived(
        Object.entries(reasons).map(([id, value]) => ({ id, value })),
    );
</script>

<MySelect.Simple
    {required}
    {options}
    bind:selected={
        () => options.find((i) => i.id === value), (v) => (value = v?.id)
    }
>
    {#snippet empty()}
        <em>You need to add a removal reason</em>
    {/snippet}

    {#snippet trigger(v)}
        {v.id}
    {/snippet}

    {#snippet item(v)}
        <strong>{v.id}</strong>: {v.value}
    {/snippet}
</MySelect.Simple>
