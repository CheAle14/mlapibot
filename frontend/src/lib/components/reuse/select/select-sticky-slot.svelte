<script lang="ts">
    import MySelect from "./select-simple.svelte";

    const OPTIONS: { id: number; name: string }[] = [
        {
            id: 0,
            name: "No sticky",
        },
        {
            id: 1,
            name: "Top slot",
        },
        {
            id: 2,
            name: "Bottom slot",
        },
    ];

    interface Props {
        value?: string | number;
        disabled?: boolean;
    }

    let { value = $bindable(), ...rest }: Props = $props();
</script>

<MySelect
    options={OPTIONS}
    bind:selected={
        () => OPTIONS[typeof value === "number" ? value : 0],
        (v) => (value = v.id)
    }
    {...rest}
>
    {#snippet item(v)}
        {v.name}
    {/snippet}
</MySelect>
