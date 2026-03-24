<script lang="ts">
    import Checkbox from "./checkbox.svelte";
    import { Label } from "$lib/components/ui/label";
    import { useId } from "bits-ui";

    interface Props {
        checked?: boolean;
        disabled?: boolean;
        label: string;
    }

    const id = useId();
    let { checked = $bindable(), label, ...rest }: Props = $props();
</script>

<div class="flex items-center">
    <Checkbox {id} bind:checked {...rest} />
    <Label class="pl-3 text-sm font-medium leading-none" for={id}>{label}</Label
    >
</div>
