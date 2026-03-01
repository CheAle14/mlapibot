<script lang="ts">
    import { useId } from "bits-ui";
    import * as Dialog from "../ui/dialog";
    import { Button } from "../ui/button";
    import TemplateModalContent from "./TemplateModalContent.svelte";
    import { getTemplateInfo } from "$lib/api/templates.remote";
    import type { EditTemplateInfo } from "$lib/types/templates";
    import { FormWrapped } from "../reuse/form";

    interface Props {
        subreddit: string;
        id: number;

        name?: string;
        content?: string;

        onSubmit(edit: EditTemplateInfo): void;
    }

    const formId = useId();
    let {
        subreddit,
        id,
        name: originalName,
        content: originalContent,
        onSubmit,
    }: Props = $props();

    let query = $derived(getTemplateInfo({ subreddit, template: id }));

    let newName = $state<string>();
    let newContent = $state<string>();

    const currentName = $derived(newName ?? originalName);
    const currentContent = $derived(
        newContent ?? originalContent ?? query.current?.content,
    );
</script>

<Dialog.Content size="large">
    <Dialog.Header>Edit Template</Dialog.Header>

    <FormWrapped
        id={formId}
        onsubmit={() =>
            onSubmit({
                id,
                name: currentName ?? "",
                content: currentContent ?? "",
            })}
    >
        {#if query.current}
            <TemplateModalContent
                bind:name={() => currentName, (v) => (newName = v)}
                bind:content={() => currentContent, (v) => (newContent = v)}
            />
        {:else}
            <p>Loading...</p>
        {/if}
    </FormWrapped>

    <Dialog.Footer>
        <Button form={formId} type="submit">Submit</Button>
    </Dialog.Footer>
</Dialog.Content>
