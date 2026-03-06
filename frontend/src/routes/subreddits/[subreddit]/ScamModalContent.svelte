<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Input, InputClearable } from "$lib/components/ui/input";
    import * as Select from "$lib/components/reuse/select";
    import type { CreateOrUpdateScamInfo } from "$lib/types/subreddit";
    import { Button } from "$lib/components/ui/button";
    import * as Field from "$lib/components/ui/field";
    import { ScamMatcher } from "$lib/components/ui/scam-matcher";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import * as Accordion from "$lib/components/ui/accordion";
    import { type IMatcher } from "$lib/types/matcher";
    import { toast } from "svelte-sonner";
    import {
        ClipboardPaste,
        ClipboardCopy,
        Shrink,
        Expand,
    } from "@lucide/svelte";
    import {
        SelectRemovalReason,
        SelectReplyTemplate,
    } from "$lib/components/reuse/select";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { useId } from "bits-ui";
    import { FormWrapped } from "$lib/components/reuse/form";

    const formId = useId();

    let fullscreenMatchers = $state(false);

    interface Props {
        subreddit: string;
        deleted_templates?: number[];
        removal_reasons: Record<string, string>;
        item: CreateOrUpdateScamInfo;
        onSubmit(updates: CreateOrUpdateScamInfo): void;
    }

    type Key = "ocr" | "title" | "body" | "title_or_body";

    type Option = {
        key: Key;
        name: string;
        description: string;
    };

    const OPTIONS: Option[] = [
        {
            key: "ocr",
            name: "OCR",
            description:
                "Search across all images in the post, downloading any links, etc",
        },
        {
            key: "title",
            name: "Title",
            description: "Search in the post's title only",
        },
        {
            key: "body",
            name: "Body",
            description: "Search in the post's body (selftext) only",
        },
        {
            key: "title_or_body",
            name: "Title + Body",
            description: "Search in body the post's title or the post's body",
        },
    ];

    let {
        subreddit,
        deleted_templates,
        removal_reasons,
        item = $bindable(),
        onSubmit,
    }: Props = $props();

    const BRAND = "mlapibot-scam";
    let copyToClipboard = () => {
        const data = {
            ...item,
            $type: BRAND,
        };

        const text = btoa(JSON.stringify(data));
        navigator.clipboard.writeText(text);

        toast.success(`Copied ${text.length} bytes`);
    };

    let pasteFromClipboard = async () => {
        const b64 = await navigator.clipboard.readText();
        const text = atob(b64);
        const data = JSON.parse(text);

        if ("$type" in data && data["$type"] === BRAND) {
            delete data["$type"];
            data.id = item.id;
            item = data;
        } else {
            toast.error("Unrecognised paste data");
        }
    };
</script>

<Dialog.Content class="h-11/12" size="full">
    <Dialog.Header>
        <Dialog.Title
            >{typeof item.id === "string" ? "Create" : "Edit"}
            rule</Dialog.Title
        >
    </Dialog.Header>
    <FormWrapped id={formId} onsubmit={() => onSubmit(item)}>
        <Field.Group>
            {#if !fullscreenMatchers}
                <Field.Set class="lg:grid lg:grid-cols-2 gap-4">
                    <Field.Field>
                        <Field.Label>Rule Name</Field.Label>
                        <Input required type="text" bind:value={item.name} />
                    </Field.Field>

                    <Field.Field>
                        <Field.Label>Reply template</Field.Label>

                        <SelectReplyTemplate
                            {subreddit}
                            {deleted_templates}
                            bind:value={item.template}
                        />

                        <Field.Description
                            >If set, which template should be used to reply. If
                            not set, no reply is sent</Field.Description
                        >
                    </Field.Field>

                    <Field.Group class="flex flex-row">
                        <Field.Group class="flex flex-col">
                            <Field.Field orientation="horizontal">
                                <Checkbox
                                    bind:checked={
                                        () => item.enabled ?? false,
                                        (v) => (item.enabled = v)
                                    }
                                />

                                <Field.Content>
                                    <Field.Label>Enabled</Field.Label>
                                    <Field.Description
                                        >If off, this rule is simply ignored</Field.Description
                                    >
                                </Field.Content>
                            </Field.Field>

                            <Field.Field orientation="horizontal">
                                <Checkbox
                                    bind:checked={
                                        () => item.self_post ?? false,
                                        (v) => (item.self_post = v)
                                    }
                                />

                                <Field.Content>
                                    <Field.Label
                                        >Run on text-only posts</Field.Label
                                    >
                                    <Field.Description
                                        >Should this rule apply to text-only
                                        (self posts)?</Field.Description
                                    >
                                </Field.Content>
                            </Field.Field>
                        </Field.Group>

                        <Field.Field>
                            <Field.Label>Mod Action</Field.Label>
                            <Select.ScamAction bind:scam={item} />
                            <Field.Description
                                >If this rule matches, what moderator action
                                should be performed.</Field.Description
                            >
                        </Field.Field>
                    </Field.Group>

                    {#if item.remove}
                        <Field.Field>
                            <Field.Label>Removal reason</Field.Label>

                            <SelectRemovalReason
                                reasons={removal_reasons}
                                bind:value={item.reason}
                            />

                            <Field.Description
                                >This alias is looked up to map to a removal
                                reason's ID. If we reply, the text of that
                                removal reason is available for the above
                                template to include.</Field.Description
                            >
                        </Field.Field>
                    {/if}
                </Field.Set>
            {/if}
            <Field.Set>
                <ScrollArea
                    class={["w-full", fullscreenMatchers ? "h-160" : "h-72"]}
                >
                    <Accordion.Root type="single">
                        {#each OPTIONS as option (option.key)}
                            <Accordion.Item value={option.key}>
                                <Accordion.Trigger
                                    >{option.name}</Accordion.Trigger
                                >
                                <Accordion.Content>
                                    <Field.Description
                                        >{option.description}</Field.Description
                                    >

                                    <ScamMatcher
                                        bind:value={
                                            item[option.key] as
                                                | IMatcher
                                                | undefined
                                        }
                                        deleteSelf={() =>
                                            (item[option.key] = undefined)}
                                    />
                                </Accordion.Content>
                            </Accordion.Item>
                        {/each}
                    </Accordion.Root>
                </ScrollArea>
            </Field.Set>
        </Field.Group>
    </FormWrapped>
    <Dialog.Footer>
        <Button
            onclick={() => (fullscreenMatchers = !fullscreenMatchers)}
            variant="outline"
            size="icon"
            title={fullscreenMatchers
                ? "Minimise matchers"
                : "Full-screen matchers"}
        >
            {#if fullscreenMatchers}
                <Shrink />
            {:else}
                <Expand />
            {/if}
        </Button>

        {#if typeof item.id === "number"}
            <Button
                onclick={copyToClipboard}
                variant="outline"
                size="icon"
                title="Copy data"
            >
                <ClipboardCopy />
            </Button>
        {/if}
        <Button
            onclick={pasteFromClipboard}
            variant="outline"
            size="icon"
            title="Paste data"><ClipboardPaste /></Button
        >

        <Button type="submit" form={formId}>Save changes</Button>
    </Dialog.Footer>
</Dialog.Content>
