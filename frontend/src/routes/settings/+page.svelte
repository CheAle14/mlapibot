<script lang="ts">
    import {
        addSubredditModerator,
        createSubreddit,
    } from "$lib/api/subreddits.remote.js";
    import { Button } from "$lib/components/ui/button";
    import * as Field from "$lib/components/ui/field";
    import { Input } from "$lib/components/ui/input";

    const random = crypto.randomUUID();

    let { data } = $props();

    $effect(() => {
        createSubreddit.fields.id.set(random);
        addSubredditModerator.fields.subreddit_id.set(random);

        if (data && data.me) {
            addSubredditModerator.fields.username.set(data.me.name);
        }
    });
</script>

<div class="flex flex-col lg:flex-row justify-around">
    <div class="w-full max-w-md">
        <form {...createSubreddit}>
            <Field.Group>
                <Field.Set>
                    <Field.Legend>Create new subreddit</Field.Legend>

                    <Field.Group>
                        <Field.Field>
                            <Field.Label>ID</Field.Label>
                            <Input {...createSubreddit.fields.id.as("text")} />
                        </Field.Field>

                        <Field.Field>
                            <Field.Label>Name</Field.Label>
                            <Input
                                {...createSubreddit.fields.name.as("text")}
                            />
                        </Field.Field>
                    </Field.Group>
                </Field.Set>
                <Button type="submit" disabled={createSubreddit.pending > 0}
                    >Submit</Button
                >
            </Field.Group>
        </form>
    </div>

    <div class="w-full max-w-md">
        <form {...addSubredditModerator}>
            <Field.Group>
                <Field.Set>
                    <Field.Legend>Add subreddit moderator</Field.Legend>

                    <Field.Group>
                        <Field.Field>
                            <Field.Label>Subreddit ID</Field.Label>
                            <Input
                                {...addSubredditModerator.fields.subreddit_id.as(
                                    "text",
                                )}
                            />
                        </Field.Field>

                        <Field.Field>
                            <Field.Label>Username</Field.Label>
                            <Input
                                {...addSubredditModerator.fields.username.as(
                                    "text",
                                )}
                            />
                        </Field.Field>
                    </Field.Group>
                </Field.Set>
                <Button
                    type="submit"
                    disabled={addSubredditModerator.pending > 0}>Submit</Button
                >
            </Field.Group>
        </form>
    </div>
</div>
