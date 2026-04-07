<script lang="ts">
    import { goto } from "$app/navigation";
    import { createSubPost } from "$lib/api/posts.remote";
    import { FormWrapped } from "$lib/components/reuse/form";
    import MarkdownContent from "$lib/components/reuse/markdown-content.svelte";
    import { SelectStickySlot } from "$lib/components/reuse/select";
    import { AlertError } from "$lib/components/ui/alert";
    import { Button } from "$lib/components/ui/button";
    import {
        Checkbox,
        CheckboxGroup,
        CheckboxLabeled,
    } from "$lib/components/ui/checkbox";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputClearable } from "$lib/components/ui/input";
    import * as Item from "$lib/components/ui/item";
    import { Json } from "$lib/components/ui/json";
    import { Spinner } from "$lib/components/ui/spinner";
    import { Textarea } from "$lib/components/ui/textarea";
    import type { CreateSubredditPost } from "$lib/types/posts";
    import type { PageProps } from "./$types";
    import Markdown from "svelte-exmarkdown";

    let { params, data }: PageProps = $props();

    let subreddit_id = $derived(
        data.subs.find((s) => s.name === params.subreddit)?.id ?? "<??>",
    );

    let values = $state<CreateSubredditPost>({
        title: "",
        content: "",
        subreddit: "",
        sticky: 0,
        distinguish: true,
        lock: true,
    });

    $effect(() => {
        values.subreddit = subreddit_id;
    });

    const toggleFlair = (v: boolean) => {
        if (v) {
            values.flair_id = "";
            values.flair_text = "";
        } else {
            values.flair_id = undefined;
            values.flair_text = undefined;
        }
    };

    const hasFlair = $derived(values.flair_id !== undefined);

    let isSubmitting = $state(false);
    let submitError = $state<any>(null);

    const handleSubmit = async () => {
        isSubmitting = true;
        submitError = null;
        try {
            const result = await createSubPost(values);
            await goto(`/subreddits/${params.subreddit}/posts/${result.id}`);
        } catch (err) {
            submitError = err;
        } finally {
            isSubmitting = false;
        }
    };
</script>

<FormWrapped onsubmit={handleSubmit}>
    <Field.Group>
        <Field.Title>Create Post</Field.Title>

        <Field.Set>
            <Field.Group class="flex xl:flex-row">
                <Field.Group>
                    <Field.Field>
                        <Field.Label>Title</Field.Label>
                        <Input type="text" required bind:value={values.title} />
                    </Field.Field>
                </Field.Group>

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
                            bind:value={values.flair_id}
                        />
                    </Field.Field>

                    <Field.Field class={[!hasFlair && "invisible"]}>
                        <Field.Label>Flair Text</Field.Label>
                        <Field.Description
                            >If omitted, the default for the flair ID is used.</Field.Description
                        >

                        <InputClearable
                            type="text"
                            coerceEmpty
                            bind:value={values.flair_text}
                        />
                    </Field.Field>
                </Field.Optional>
            </Field.Group>

            <Field.Group class="flex xl:flex-row">
                <Field.Field class="flex-1/2">
                    <Field.Label>Sticky</Field.Label>

                    <SelectStickySlot bind:value={values.sticky} />
                </Field.Field>

                <CheckboxGroup label="Options" class="flex-1/2">
                    <CheckboxLabeled
                        label="Distinguish"
                        bind:checked={values.distinguish}
                    />
                    <CheckboxLabeled label="Lock" bind:checked={values.lock} />
                </CheckboxGroup>
            </Field.Group>

            <MarkdownContent bind:content={values.content}>
                {#snippet buttons()}
                    <Button
                        type="submit"
                        pending={isSubmitting}
                        errored={submitError !== null}>Create draft</Button
                    >

                    {#if submitError}
                        <AlertError
                            title="Failed to create"
                            error={submitError}
                        />
                    {/if}
                {/snippet}
            </MarkdownContent>
        </Field.Set>
    </Field.Group>
</FormWrapped>
