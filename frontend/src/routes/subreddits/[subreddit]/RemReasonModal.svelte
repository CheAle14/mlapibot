<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Input } from "$lib/components/ui/input";
    import { Button } from "$lib/components/ui/button";
    import * as Field from "$lib/components/ui/field";

    export interface RemReasonItem {
        adding: boolean;
        key: string;
        value: string;
    }

    interface Props {
        item: RemReasonItem;
        onSubmit(item: RemReasonItem): void;
    }

    let { item = $bindable(), onSubmit }: Props = $props();
</script>

<Dialog.Content>
    <Dialog.Header>
        {#if item.adding}
            Create Alias
        {:else}
            Edit {item.key}
        {/if}
    </Dialog.Header>
    <Field.Group>
        <Field.Set>
            <Field.Group>
                <Field.Field>
                    <Field.Label>Alias</Field.Label>

                    <Input
                        type="text"
                        bind:value={item.key}
                        disabled={!item.adding}
                    />
                </Field.Field>

                <Field.Field>
                    <Field.Label>Removal Reason UUID</Field.Label>
                    <Input type="text" bind:value={item.value} />
                </Field.Field>
            </Field.Group>
        </Field.Set>
    </Field.Group>
    <Dialog.Footer>
        <Button type="submit" onclick={() => onSubmit(item)}
            >Save changes</Button
        >
    </Dialog.Footer>
</Dialog.Content>
