<script lang="ts">
    import type { Snippet } from "svelte";
    import { Field, FieldContent, FieldGroup, FieldLabel, FieldSet } from ".";
    import { Checkbox } from "../checkbox";

    interface Props {
        trigger: Snippet<[]>;
        children?: Snippet<[]>;
        open?: boolean;
    }

    let { open = $bindable(false), trigger, children }: Props = $props();
</script>

<FieldGroup class="flex flex-col">
    <Field orientation="horizontal">
        <Checkbox bind:checked={open} />

        <FieldContent>
            {@render trigger()}
        </FieldContent>
    </Field>

    <FieldSet class={open ? "" : "invisible"}>
        {@render children?.()}
    </FieldSet>
</FieldGroup>
