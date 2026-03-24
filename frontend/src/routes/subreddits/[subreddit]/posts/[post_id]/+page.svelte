<script lang="ts">
    import { goto } from "$app/navigation";
    import {
        deleteSubPost,
        publishSubPost,
        updateSubPost,
    } from "$lib/api/posts.remote";
    import { FormWrapped } from "$lib/components/reuse/form";
    import MarkdownContent from "$lib/components/reuse/markdown-content.svelte";
    import { SelectStickySlot } from "$lib/components/reuse/select";
    import * as Alert from "$lib/components/ui/alert";
    import { Button } from "$lib/components/ui/button";
    import ButtonConfirm from "$lib/components/ui/button/button-confirm.svelte";
    import {
        CheckboxGroup,
        CheckboxLabeled,
    } from "$lib/components/ui/checkbox";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputClearable } from "$lib/components/ui/input";
    import { Json } from "$lib/components/ui/json";
    import type { SubredditPost } from "$lib/types/posts";
    import type { PageProps } from "./$types";

    let { params, data }: PageProps = $props();

    let subreddit_id = $derived(data.subreddit_id);
    let changes = $state<SubredditPost>(data.post);

    $effect(() => {
        changes = data.post;
    });

    const isPublished = $derived(!!changes.reddit_id);

    const toggleFlair = (v: boolean) => {
        console.log("toggle", v);
        if (v) {
            changes.flair_id = "";
            changes.flair_text = "";
        } else {
            changes.flair_id = undefined;
            changes.flair_text = undefined;
        }
    };

    const hasFlair = $derived(
        changes.flair_id !== undefined && changes.flair_id !== null,
    );

    let isSubmitting = $state(false);
    let submitError = $state<any>(null);
    let isPublishing = $state(false);
    let publishError = $state<any>(null);
    let isDeleting = $state(false);
    let deleteError = $state<any>(null);

    const handleSubmit = async () => {
        isSubmitting = true;
        submitError = null;
        try {
            const result = await updateSubPost(changes);
            changes = result;
        } catch (err) {
            submitError = err;
        } finally {
            isSubmitting = false;
        }
    };

    const handlePublish = async () => {
        isPublishing = true;
        publishError = null;
        try {
            const { id } = await publishSubPost({
                subreddit_id,
                post_id: changes.id,
            });
            changes.synced_at = changes.updated_at;
            changes.reddit_id = id;
        } catch (err) {
            publishError = err;
        } finally {
            isPublishing = false;
        }
    };

    const handleDelete = async () => {
        isDeleting = true;
        deleteError = null;
        try {
            await deleteSubPost({
                subreddit_id,
                post_id: changes.id,
            });
            await goto(`/subreddits/${params.subreddit}/posts`);
        } catch (err) {
            deleteError = err;
        } finally {
            isDeleting = false;
        }
    };
</script>

<FormWrapped onsubmit={handleSubmit}>
    <Field.Group>
        <Field.Title>Edit Post</Field.Title>

        <Field.Set>
            <Field.Group class="flex xl:flex-row">
                <Field.Group>
                    <Field.Field>
                        <Field.Label>Title</Field.Label>
                        <Input
                            type="text"
                            required
                            disabled={isPublished}
                            bind:value={changes.title}
                        />
                    </Field.Field>
                </Field.Group>

                {#if isPublished}
                    <Field.Field>
                        <Field.Label>Reddit Post</Field.Label>

                        <Field.Description>
                            <a
                                href={`https://reddit.com/r/${params.subreddit}/comments/${changes.reddit_id}`}
                                >This post has been published</a
                            >, so some settings cannot be changed.
                        </Field.Description>
                    </Field.Field>
                {:else}
                    <Field.Optional bind:open={() => hasFlair, toggleFlair}>
                        {#snippet trigger()}
                            <Field.Label>Set post flair</Field.Label>
                        {/snippet}

                        <Field.Field class={[!hasFlair && "invisible"]}>
                            <Field.Label>Flair ID</Field.Label>
                            <Field.Description
                                >Can be copied from <a
                                    href={`https://sh.reddit.com/mod/${params.subreddit}/postflair`}
                                    >mod tools</a
                                >.</Field.Description
                            >

                            <InputClearable
                                type="text"
                                coerceEmpty
                                bind:value={changes.flair_id}
                            />
                        </Field.Field>

                        <Field.Field class={[!hasFlair && "invisible"]}>
                            <Field.Label>Flair Text</Field.Label>
                            <Field.Description
                                >If omitted, the default for the flair ID is
                                used.</Field.Description
                            >

                            <InputClearable
                                type="text"
                                coerceEmpty
                                bind:value={changes.flair_text}
                            />
                        </Field.Field>
                    </Field.Optional>
                {/if}
            </Field.Group>

            <Field.Group class="flex xl:flex-row">
                <Field.Field class="flex-1/2">
                    <Field.Label>Sticky</Field.Label>

                    <SelectStickySlot
                        disabled={isPublished}
                        bind:value={changes.sticky}
                    />
                </Field.Field>

                <CheckboxGroup label="Options" class="flex-1/2">
                    <CheckboxLabeled
                        label="Distinguish"
                        disabled={isPublished}
                        bind:checked={changes.distinguish}
                    />
                    <CheckboxLabeled
                        label="Lock"
                        disabled={isPublished}
                        bind:checked={changes.lock}
                    />
                </CheckboxGroup>
            </Field.Group>

            <MarkdownContent bind:content={changes.content}>
                {#snippet buttons()}
                    <Button
                        class="w-full"
                        type="submit"
                        pending={isSubmitting}
                        errored={submitError !== null}
                    >
                        Save changes
                    </Button>

                    {#if isPublished}
                        {#if changes.updated_at !== changes.synced_at}
                            <Alert.Root>
                                <Alert.Title>Sync changes</Alert.Title>
                                <Alert.Description>
                                    There are previous changes that have not
                                    been synced to the Reddit post. If you have
                                    made any further changes, save them above
                                    and then sync.

                                    <Button
                                        class="w-full"
                                        variant="destructive"
                                        type="button"
                                        onclick={handlePublish}
                                        pending={isPublishing}
                                        errored={publishError !== null}
                                    >
                                        Sync changes
                                    </Button>
                                </Alert.Description>
                            </Alert.Root>
                        {/if}
                    {:else}
                        <ButtonConfirm
                            description="This will submit this post to the subreddit, an action that cannot be reversed."
                            class="w-full"
                            variant="destructive"
                            type="button"
                            pending={isPublishing}
                            errored={publishError !== null}
                            onclick={handlePublish}
                            >Publish
                        </ButtonConfirm>

                        <ButtonConfirm
                            description="This will permanently delete this draft"
                            class="w-full"
                            variant="destructive"
                            type="button"
                            pending={isDeleting}
                            errored={deleteError !== null}
                            onclick={handleDelete}
                            >Delete draft
                        </ButtonConfirm>
                    {/if}

                    {#if submitError}
                        <Alert.Error
                            title="Failed to save"
                            error={submitError}
                        />
                    {/if}

                    {#if publishError}
                        <Alert.Error
                            title="Failed to publish"
                            error={publishError}
                        />
                    {/if}

                    {#if deleteError}
                        <Alert.Error
                            title="Failed to delete"
                            error={deleteError}
                        />
                    {/if}
                {/snippet}
            </MarkdownContent>
        </Field.Set>
    </Field.Group>
</FormWrapped>
