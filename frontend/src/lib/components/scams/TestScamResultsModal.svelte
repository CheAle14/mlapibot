<script lang="ts">
    import type { GotAnalysis, PostAction } from "$lib/types/api";
    import ArrowUp_0_1 from "@lucide/svelte/icons/arrow-up-0-1";
    import * as Accordion from "../ui/accordion";
    import * as Dialog from "../ui/dialog";
    import {
        Field,
        FieldDescription,
        FieldGroup,
        FieldLabel,
    } from "../ui/field";
    import { Json } from "../ui/json";
    import { ScrollArea } from "../ui/scroll-area";
    import OcrImageText from "./OcrImageText.svelte";
    import { Badge } from "../ui/badge";

    interface Props {
        results: GotAnalysis;
    }

    let { results }: Props = $props();
    let { action, ocr } = $derived(results);
</script>

<Dialog.Content size="large">
    {#if action.type === "ignore"}
        <Dialog.Header>Ignore</Dialog.Header>
        <p>No rule was triggered, so no action would be taken taken</p>
    {:else}
        <Dialog.Header>Action</Dialog.Header>

        <FieldGroup class="flex flex-row justify-around">
            <Field>
                <FieldLabel>Rule</FieldLabel>
                <FieldDescription
                    ><strong>{action.analyser}</strong> was triggered</FieldDescription
                >
            </Field>

            <Field>
                <FieldLabel>Moderation</FieldLabel>
                <FieldDescription>
                    {#if action.moderate === "none"}
                        No moderator action taken
                    {:else}
                        The post will be <strong
                            >{action.moderate.endsWith("e")
                                ? action.moderate + "d"
                                : action.moderate + "ed"}</strong
                        >

                        {#if action.reason}
                            with reason <em>{action.reason}</em>
                        {/if}
                    {/if}
                </FieldDescription>
            </Field>
        </FieldGroup>
    {/if}

    <ScrollArea class="max-h-125 pr-3">
        <Accordion.Root type="single">
            {#each ocr as image}
                <Accordion.Item value={image.name}>
                    <Accordion.Trigger>
                        {#if image.triggers.length > 0}
                            <Badge>{image.triggers.length}</Badge>
                        {/if}

                        {image.name}
                    </Accordion.Trigger>
                    <Accordion.Content>
                        {#if image.triggers.length === 0}
                            <pre class="whitespace-pre-line">
                                    {image.text}
                            </pre>
                        {:else}
                            <OcrImageText
                                text={image.text}
                                triggers={image.triggers}
                            />
                        {/if}
                    </Accordion.Content>
                </Accordion.Item>
            {/each}

            {#if action.type === "action" && action.reply}
                <Accordion.Item value="::reply">
                    <Accordion.Trigger
                        >{action.reply.distinguish ? "Distinguished" : ""} Reply</Accordion.Trigger
                    >
                    <Accordion.Content>
                        <pre class="whitespace-pre-line">
                    {action.reply.text}
                </pre>
                    </Accordion.Content>
                </Accordion.Item>
            {/if}
        </Accordion.Root>
    </ScrollArea>
</Dialog.Content>
