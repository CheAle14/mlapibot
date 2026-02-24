<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Input } from "$lib/components/ui/input";
    import * as Select from "$lib/components/reuse/select";
    import type { ScamInfo } from "$lib/types/subreddit";
    import { Button } from "$lib/components/ui/button";

    interface Props {
        item: ScamInfo;

        onSubmit(updates: ScamInfo): void;
    }

    let { item = $bindable(), onSubmit }: Props = $props();
</script>

<Dialog.Content>
    <Dialog.Header>
        <Dialog.Title>Edit {item.name}</Dialog.Title>
    </Dialog.Header>
    <div>
        <Input type="text" bind:value={item.name} />
        <Select.ScamAction bind:scam={item} />
    </div>
    <Dialog.Footer>
        <Button type="submit" onclick={() => onSubmit(item)}
            >Save changes</Button
        >
    </Dialog.Footer>
</Dialog.Content>
