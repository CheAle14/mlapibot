<script lang="ts">
    import { SendHorizonal } from "@lucide/svelte";
    import { Button } from "../ui/button";
    import * as Dialog from "../ui/dialog";
    import * as Field from "../ui/field";
    import { Input, InputList } from "../ui/input";
    import * as Tabs from "../ui/tabs";
    import { FormWrapped } from "../reuse/form";
    import { useId } from "bits-ui";
    import { Textarea } from "../ui/textarea";
    import {
        fetchRedditSubmission,
        fetchScamAnalysisResult,
    } from "$lib/api/scams.remote";
    import type {
        GotAnalysis,
        GotRedditPost,
        PostAction,
    } from "$lib/types/api";
    import TestScamResultsModal from "./TestScamResultsModal.svelte";
    import * as Alert from "../ui/alert";

    interface Props {
        subreddit_id: string;
        onClose(): void;
    }

    type State =
        | {
              state: "reddit";
              link: string;
          }
        | ({
              state: "manual";
          } & GotRedditPost);

    let { subreddit_id, onClose }: Props = $props();

    let firstModal = $state<State>({
        state: "reddit",
        link: "",
    });

    let secondModal = $state<undefined | GotAnalysis>(undefined);

    let isPending = $state(false);
    let isError = $state(false);

    const setTab = (value: string) => {
        if (value === "reddit") {
            firstModal = {
                state: "reddit",
                link: "",
            };
        } else {
            firstModal = {
                state: "manual",
                author: "",
                id: "",
                subreddit_id,
                title: "",
                body: "",
            };
        }
    };

    const doRedditLink = async (link: string) => {
        isPending = true;
        isError = false;
        let post;
        try {
            post = await fetchRedditSubmission(link);
        } catch {
            isError = true;
            return;
        } finally {
            isPending = false;
        }

        firstModal = {
            state: "manual",
            ...post,
            subreddit_id,
        };
    };

    const runTest = async (post: GotRedditPost) => {
        console.log(post);
        isPending = true;
        isError = false;
        try {
            secondModal = await fetchScamAnalysisResult(post);
        } catch {
            isError = true;
        } finally {
            isPending = false;
        }
    };

    const redditForm = useId();
    const manualForm = useId();
</script>

<Dialog.Content size={firstModal.state === "reddit" ? "small" : "medium"}>
    <Dialog.Root
        bind:open={
            () => secondModal !== undefined, () => (secondModal = undefined)
        }
    >
        {#if secondModal}
            <TestScamResultsModal results={secondModal} />
        {/if}
    </Dialog.Root>

    <Dialog.Header>Test current OCR rules</Dialog.Header>

    <Tabs.Root bind:value={() => firstModal.state, setTab}>
        <Tabs.List>
            <Tabs.Trigger value="reddit">Reddit</Tabs.Trigger>
            <Tabs.Trigger value="manual">Manual</Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="reddit">
            {#if firstModal.state === "reddit"}
                <FormWrapped
                    id={redditForm}
                    onsubmit={() =>
                        doRedditLink(
                            firstModal.state === "reddit"
                                ? firstModal.link
                                : "",
                        )}
                >
                    <Field.Group>
                        <Field.Set>
                            <Field.Field>
                                <Field.Label>Submission link</Field.Label>

                                <div class="flex gap-2">
                                    <Input
                                        required
                                        type="url"
                                        bind:value={firstModal.link}
                                    />

                                    <Button
                                        pending={isPending}
                                        errored={isError}
                                        form={redditForm}
                                        type="submit"
                                        size="icon"><SendHorizonal /></Button
                                    >
                                </div>
                            </Field.Field>
                        </Field.Set>
                    </Field.Group>
                </FormWrapped>
            {/if}

            <Alert.Root class="mt-2">
                <Alert.Description
                    ><p>
                        This only tests against the <strong>current</strong>
                        rules,
                        <em>not</em> any pending changes you may have made
                    </p></Alert.Description
                >
            </Alert.Root>
        </Tabs.Content>

        <Tabs.Content value="manual">
            {#if firstModal.state === "manual"}
                <FormWrapped
                    id={manualForm}
                    onsubmit={() => runTest(firstModal as GotRedditPost)}
                >
                    <Field.Group>
                        <Field.Set>
                            <Field.Field>
                                <Field.Label>Subreddit ID</Field.Label>
                                <Field.Description
                                    >Which subreddit's rules should be used.
                                    Defaults to this one.</Field.Description
                                >

                                <Input
                                    type="text"
                                    bind:value={firstModal.subreddit_id}
                                />
                            </Field.Field>

                            <Field.Field>
                                <Field.Label>Title</Field.Label>

                                {#if firstModal.author}
                                    <Field.Description
                                        >By /u/{firstModal.author}</Field.Description
                                    >
                                {/if}

                                <Input
                                    type="text"
                                    bind:value={firstModal.title}
                                />
                            </Field.Field>

                            <Field.Field>
                                <Field.Label>Link</Field.Label>
                                <InputList
                                    type="url"
                                    popoverClass="w-xl"
                                    bind:value={firstModal.links}
                                />
                            </Field.Field>

                            <Field.Field>
                                <Field.Label>Body / Self text</Field.Label>
                                <Textarea bind:value={firstModal.body} />
                            </Field.Field>
                        </Field.Set>
                    </Field.Group>
                </FormWrapped>
            {/if}
        </Tabs.Content>
    </Tabs.Root>

    {#if firstModal.state === "manual"}
        <Dialog.Footer>
            <Button
                type="button"
                pending={isPending}
                errored={isError}
                onclick={() => runTest(firstModal as GotRedditPost)}
                >Submit</Button
            >
        </Dialog.Footer>
    {/if}
</Dialog.Content>
