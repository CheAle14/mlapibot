<script lang="ts">
    import { FormWrapped } from "$lib/components/reuse/form";
    import { RemovalReason } from "$lib/components/reuse/select";
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputList } from "$lib/components/ui/input";
    import { Kbd } from "$lib/components/ui/kbd";
    import type { ComplexCommentModalItem } from "$lib/types/subreddit";
    import { useId } from "bits-ui";

    interface Props {
        removal_reasons: Record<string, string>;
        item: ComplexCommentModalItem;

        onSubmit(item: ComplexCommentModalItem): void;
    }

    const formId = useId();
    let { removal_reasons, onSubmit, item = $bindable() }: Props = $props();
</script>

<Dialog.Content size="large">
    <Dialog.Header>Edit Complex Comment</Dialog.Header>

    <FormWrapped id={formId} onsubmit={() => onSubmit(item)}>
        <Field.Group class="flex-col">
            <Field.Set class="flex-row">
                <Field.Field>
                    <Field.Label>Name</Field.Label>
                    <Input required type="text" bind:value={item.name} />
                </Field.Field>

                <Field.Field>
                    <Field.Label>Removal Reason</Field.Label>

                    <RemovalReason
                        required
                        reasons={removal_reasons}
                        bind:value={item.reason}
                    />
                </Field.Field>
            </Field.Set>

            <Field.Description class="w-full text-center">
                A comment is removed if it
                <Kbd>1</Kbd> contains a specified phrase, within a post whose title
                <Kbd>2</Kbd> contains the specified text, and if the post's title
                is <Kbd>3</Kbd> not ignored.
            </Field.Description>

            <Field.Set class="flex-row">
                <Field.Field>
                    <Field.Label><Kbd>1</Kbd> Comment text</Field.Label>
                    <InputList type="text" bind:value={item.comment} />
                </Field.Field>
                <Field.Field>
                    <Field.Label><Kbd>2</Kbd> Title text</Field.Label>
                    <InputList type="text" bind:value={item.link_title} />
                </Field.Field>
                <Field.Field>
                    <Field.Label><Kbd>3</Kbd> Ignored flairs</Field.Label>
                    <InputList type="text" bind:value={item.ignore_flairs} />
                </Field.Field>
            </Field.Set>
        </Field.Group>
    </FormWrapped>

    <Dialog.Footer>
        <Button form={formId} type="submit">Submit</Button>
    </Dialog.Footer>
</Dialog.Content>
