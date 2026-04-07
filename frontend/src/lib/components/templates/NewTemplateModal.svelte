<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import type { CreateTemplateInfo } from "$lib/types/templates";
    import { useId } from "bits-ui";
    import TemplateModalContent from "./TemplateModalContent.svelte";
    import { FormWrapped } from "../reuse/form";

    interface Props {
        id: string;
        name: string;
        content: string;

        onSubmit(template: CreateTemplateInfo): void;
    }

    const id = useId();

    let { id: template_id, name, content, onSubmit }: Props = $props();
</script>

<Dialog.Content size="large">
    <Dialog.Header
        >{id === undefined ? "Create" : "Edit"} New Template</Dialog.Header
    >

    <FormWrapped
        {id}
        onsubmit={() =>
            onSubmit({
                id: template_id,
                name: name ?? "",
                content: content ?? "",
            })}
    >
        <TemplateModalContent bind:name bind:content />
    </FormWrapped>

    <Dialog.Footer>
        <Button type="submit" form={id}>Submit</Button>
    </Dialog.Footer>
</Dialog.Content>
