<script lang="ts">
    import type { PostAction } from "$lib/types/api";
    import * as Dialog from "../ui/dialog";
    import { Json } from "../ui/json";
    import { ScrollArea } from "../ui/scroll-area";

    interface Props {
        results: PostAction;
    }

    let { results }: Props = $props();
</script>

<Dialog.Content size="large">
    {#if results.type === "ignore"}
        <Dialog.Header>Ignore</Dialog.Header>
        <p>No rule was triggered, so no action would be taken taken</p>
    {:else}
        <Dialog.Header>Action</Dialog.Header>
        <p>
            The <strong>{results.analyser}</strong> rule was triggered.
        </p>

        {#if results.moderate !== "none"}
            <p>
                The post will be <strong
                    >{results.moderate.endsWith("e")
                        ? results.moderate + "d"
                        : results.moderate + "ed"}</strong
                >
            </p>
        {/if}

        {#if results.reply}
            <p>
                The following {results.reply.distinguish ? "distinguished" : ""} reply
                will be made:
            </p>

            <ScrollArea class="w-full min-h-12 max-h-72">
                <div class="whitespace-pre-line">
                    {results.reply.text}
                </div>
            </ScrollArea>
        {/if}
    {/if}
</Dialog.Content>
