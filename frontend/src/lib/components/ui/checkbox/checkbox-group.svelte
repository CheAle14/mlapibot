<script lang="ts">
    import { cn } from "$lib/utils";
    import { Checkbox as CheckboxPrimitive, type WithChildren } from "bits-ui";
    import Label from "../label/label.svelte";

    interface Props extends WithChildren {
        class?: string;
        label?: string;
    }

    let { label, children, class: c }: Props = $props();
</script>

<div class={cn("flex flex-col gap-3", c)}>
    <Label>{label}</Label>

    <div class="flex flex-col gap-4">
        {@render children?.()}
    </div>
</div>
