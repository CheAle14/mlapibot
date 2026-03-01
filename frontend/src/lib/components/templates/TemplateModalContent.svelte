<script lang="ts">
    import * as Field from "$lib/components/ui/field";
    import { Input } from "$lib/components/ui/input";
    import { Textarea } from "$lib/components/ui/textarea";

    interface Props {
        name?: string;
        content?: string;
    }

    let { name = $bindable(), content = $bindable() }: Props = $props();
</script>

<Field.Group>
    <Field.Set>
        <Field.Field>
            <Field.Label>Name</Field.Label>

            <Input type="text" bind:value={name} required />
        </Field.Field>

        <Field.Field>
            <Field.Label>Content</Field.Label>
            <Field.Description>
                These are <a
                    tabindex={-1}
                    href="https://keats.github.io/tera/docs/#templates"
                    >Tera templates</a
                > and use their syntax accordingly.
            </Field.Description>

            <Textarea bind:value={content} required />
        </Field.Field>
    </Field.Set>
</Field.Group>
