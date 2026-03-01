<script lang="ts">
    import * as Field from "$lib/components/ui/field";
    import { Input } from "$lib/components/ui/input";

    import { createSubreddit } from "$lib/mutations/subreddits";
    import { createMutation } from "@tanstack/svelte-query";

    const random = crypto.randomUUID();

    let { data } = $props();

    let newsub = $state({
        id: random,
        name: "",
    });

    let addmod = $state({
        subreddit_id: random,
        user_id: "",
    });

    $effect(() => {
        if (data?.me?.id) {
            addmod.user_id = data.me.id;
        }
    });

    const createSub = createMutation(() => ({
        mutationKey: ["settings", "createSub"],
        mutationFn: createSubreddit,
    }));

    const addMod = createMutation(() => ({
        mutationKey: ["settings", "addMod"],
        mutationFn: createSubreddit,
    }));
</script>

<div class="w-full max-w-md">
    <form
        onsubmit={(e) => {
            e.preventDefault();
            createSub.mutate(newsub);
            return false;
        }}
    >
        <Field.Group>
            <Field.Set>
                <Field.Legend>Create new subreddit</Field.Legend>

                <Field.Group>
                    <Field.Field>
                        <Field.Label>ID</Field.Label>
                        <Input type="text" bind:value={newsub.id} />
                    </Field.Field>

                    <Field.Field>
                        <Field.Label>Name</Field.Label>
                        <Input type="text" bind:value={newsub.name} />
                    </Field.Field>
                </Field.Group>
            </Field.Set>
            <button type="submit">Submit</button>
        </Field.Group>
    </form>
</div>

<div class="w-full max-w-md">
    <form
        onsubmit={(e) => {
            e.preventDefault();
            createSub.mutate(newsub);
            return false;
        }}
    >
        <Field.Group>
            <Field.Set>
                <Field.Legend>Add subreddit moderator</Field.Legend>

                <Field.Group>
                    <Field.Field>
                        <Field.Label>Subreddit ID</Field.Label>
                        <Input type="text" bind:value={addmod.subreddit_id} />
                    </Field.Field>

                    <Field.Field>
                        <Field.Label>User ID</Field.Label>
                        <Input type="text" bind:value={addmod.user_id} />
                    </Field.Field>
                </Field.Group>
            </Field.Set>
            <button type="submit">Submit</button>
        </Field.Group>
    </form>
</div>
