<script lang="ts">
    import * as _ from "moderndash";
    import type { PageProps } from "./$types";
    import * as Accordion from "$lib/components/ui/accordion";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputClearable } from "$lib/components/ui/input";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { Switch } from "$lib/components/ui/switch";
    import {
        ModuleKeys,
        type PendingSubredditOptions,
        type ScamInfo,
        type SubredditOptions,
    } from "$lib/types/subreddit";
    import { Json, JsonMany } from "$lib/components/ui/json";
    import Module from "./Module.svelte";
    import { SelectIncidentImpact } from "$lib/components/reuse/select";
    import { Button } from "$lib/components/ui/button";
    import ScamsTable from "./ScamsTable.svelte";
    import {
        createMutation,
        createQuery,
        useQueryClient,
    } from "@tanstack/svelte-query";
    import { syncPendingChanges } from "$lib/mutations/subreddits";
    import {
        fetchSubredditOptions,
        fetchSubredditScams,
    } from "$lib/queries/subreddits";
    import * as Spinner from "$lib/components/ui/spinner";
    import RemovalReasons from "./RemovalReasons.svelte";

    const client = useQueryClient();
    const { params, data }: PageProps = $props();

    const subreddit = $derived(
        data.subs.find((s) => s.name === params.subreddit)?.id ?? "???",
    );

    const fetchQuery = createQuery(() => ({
        queryKey: ["subreddits", subreddit],
        queryFn: () => fetchSubredditOptions(subreddit),
    }));

    const { isFetching, error, data: options } = $derived(fetchQuery);

    let open: string[] = $state([]);
    let changes = $state<PendingSubredditOptions>({
        seq_num: -1,
        removal_reasons: {},
        ai_slop: {},
        staff_reply: {},
        status: {},
        scams: {},
        related_title: {},
        complex_comments: {},
        comments_code: {},
        comments_cdn: {},
    });

    $effect(() => {
        if (options) {
            changes.seq_num = options.seq_num;
        }
    });

    const revertPendingChanges = () => {
        for (const key of ModuleKeys) {
            changes[key] = {};
        }
        changes.removal_reasons = {};
    };

    const savePendingChanges = createMutation(() => ({
        mutationFn: syncPendingChanges,
        onSuccess: (data: SubredditOptions, vars) => {
            client.setQueryData(
                ["subreddits", vars.subreddit],
                (old: SubredditOptions) => {
                    console.log("update", data, old);
                    return { ...old, ...data };
                },
            );
            client.invalidateQueries({
                queryKey: ["subreddits", subreddit, "scams"],
            });
            revertPendingChanges();
        },
    }));

    const hasChanges = $derived.by(() => {
        for (const key of ModuleKeys) {
            const value = changes[key];
            if (!_.isEqual(value, {})) {
                return true;
            }
        }

        if (!_.isEqual(changes.removal_reasons, {})) return true;

        return false;
    });

    const toggleSticky = (v: boolean) => {
        console.log("set toggle:", v);
        if (v) {
            changes.status.sticky = {
                comment_threshold: 10,
                delay_major_mins: 15,
                delay_minor_mins: 180,
            };
        } else {
            changes.status.sticky = undefined;
        }
    };

    const disableAll = () => {
        for (const key of ModuleKeys) {
            changes[key].enabled = false;
        }
    };

    const removal_reasons = $derived.by(() => {
        if (options) {
            const newreasons = {
                ...options.removal_reasons,
            };

            for (const id of changes.removal_reasons.remove ?? []) {
                delete newreasons[id];
            }

            for (const id in changes.removal_reasons.update) {
                newreasons[id] = changes.removal_reasons.update[id];
            }

            return newreasons;
        } else {
            return {};
        }
    });
</script>

<h2>
    /r/{params.subreddit} (t5_{subreddit}) {#if options}[seq-{options.seq_num}]{/if}
</h2>

<main class="w-11/12">
    <div class="flex flex-row justify-around">
        <Button
            disabled={!hasChanges}
            pending={savePendingChanges.isPending}
            errored={savePendingChanges.isError}
            onclick={() => {
                savePendingChanges.mutate({
                    subreddit,
                    changes,
                });
            }}>Save changes</Button
        >

        <Button disabled={hasChanges} variant="destructive" onclick={disableAll}
            >DISABLE ALL</Button
        >

        <Button disabled={!hasChanges} onclick={revertPendingChanges}
            >Revert changes</Button
        >
    </div>

    <div class="flex flex-row gap-1">
        <Json value={changes} title="Pending Changes" />
    </div>

    {#if isFetching}
        <Spinner.Badge>Fetching subreddit options</Spinner.Badge>
    {/if}

    {#if error}
        <Json value={error} title="Error fetching data" />
    {/if}

    {#if options}
        <Accordion.Root type="multiple" class="" bind:value={open}>
            <Accordion.Item value="removal_reasons">
                <Accordion.Trigger>Removal reasons map</Accordion.Trigger>

                <Accordion.Content class="pl-2">
                    <RemovalReasons
                        reasons={options.removal_reasons}
                        bind:updates={changes.removal_reasons.update}
                        bind:deletes={changes.removal_reasons.remove}
                    />
                </Accordion.Content>
            </Accordion.Item>

            <Module
                key="scams"
                title="Remove posts based on OCR or text content"
                {options}
                {open}
                bind:changes
            >
                {#snippet children({ open, current, pending, original })}
                    <!-- <JsonMany
                        items={[current, pending, original]}
                        titles={["current", "pending", "original"]}
                    /> -->

                    {#if open}
                        <ScamsTable
                            {removal_reasons}
                            updates={current.update}
                            creates={current.create}
                            deletes={current.deletes}
                            subreddit_id={subreddit}
                            deleteScam={(id) => {
                                pending.deletes = [
                                    ...(pending.deletes ?? []),
                                    id,
                                ];
                            }}
                            undeleteScam={(id) => {
                                pending.deletes = (
                                    pending.deletes ?? []
                                ).filter((s) => s !== id);
                            }}
                            uncreateScam={(id) => {
                                pending.create = (pending.create ?? []).filter(
                                    (s) => s.id !== id,
                                );
                            }}
                            createScam={(scam) => {
                                pending.create = [
                                    ...(pending.create ?? []),
                                    scam,
                                ];
                            }}
                            updateScam={({ id, ...changes }) => {
                                if (pending.update) {
                                    const idx = pending.update.findIndex(
                                        (s) => "id" in s && s.id === id,
                                    );

                                    if (idx !== -1) {
                                        pending.update[idx] = {
                                            ...pending.update[idx],
                                            ...changes,
                                        };
                                    } else {
                                        pending.update.push({
                                            id,
                                            ...changes,
                                        });
                                    }
                                } else {
                                    pending.update = [{ id, ...changes }];
                                }
                            }}
                        />
                    {/if}
                {/snippet}
            </Module>

            <Module
                key="status"
                title="Discord status incidents"
                {options}
                {open}
                bind:changes
            >
                {#snippet children({ current, pending })}
                    <Field.Group class="flex flex-col lg:flex-row">
                        <Field.Set>
                            <Field.Legend>Incident post</Field.Legend>
                            <Field.Description
                                >Settings for when and how an incident post is
                                made</Field.Description
                            >
                            <Field.Field>
                                <Field.Label for="min_impact"
                                    >Minimum Impact</Field.Label
                                >
                                <Field.Description
                                    >Incidents below this impact will not be
                                    posted</Field.Description
                                >

                                <SelectIncidentImpact
                                    bind:value={
                                        () => current.min_impact,
                                        (v) => (pending.min_impact = v)
                                    }
                                />
                            </Field.Field>
                            <Field.Field orientation="horizontal">
                                <Checkbox
                                    id="distinguish"
                                    bind:checked={
                                        () => current.distinguish,
                                        (v) => (pending.distinguish = v)
                                    }
                                />

                                <Field.Content>
                                    <Field.Label for="distinguish"
                                        >Distinguish</Field.Label
                                    >
                                    <Field.Description
                                        >Whether the post should be
                                        distinguished as a moderator</Field.Description
                                    >
                                </Field.Content>
                            </Field.Field>
                        </Field.Set>

                        <Field.Set>
                            <Field.Field orientation="horizontal">
                                <Switch
                                    id="sticky"
                                    bind:checked={
                                        () => !!current.sticky, toggleSticky
                                    }
                                />

                                <Field.Content>
                                    <Field.Label for="sticky"
                                        >Sticky incident posts</Field.Label
                                    >
                                    <Field.Description
                                        >Should incident posts be stickied,
                                        subject to the settings indicated.</Field.Description
                                    >
                                </Field.Content>
                            </Field.Field>

                            {#if current.sticky}
                                <Field.Field>
                                    <Field.Label for="sticky.min_impact"
                                        >Minimum Impact</Field.Label
                                    >
                                    <Field.Description
                                        >Incidents below this impact will not be
                                        stickied</Field.Description
                                    >

                                    <SelectIncidentImpact
                                        bind:value={
                                            () =>
                                                current.sticky?.min_impact ??
                                                "none",
                                            (v) =>
                                                _.set(
                                                    pending,
                                                    "sticky.min_impact",
                                                    v,
                                                )
                                        }
                                    />
                                </Field.Field>

                                <Field.Field>
                                    <Field.Label for="comment_threshold"
                                        >Comment threshold</Field.Label
                                    >
                                    <Field.Description
                                        >The threshold that determines whether a
                                        post is 'minor' or 'major', for the two
                                        following settings</Field.Description
                                    >

                                    <Input
                                        type="number"
                                        bind:value={
                                            () =>
                                                current.sticky
                                                    ?.comment_threshold ?? 0,
                                            (v) =>
                                                _.set(
                                                    pending,
                                                    "sticky.comment_threshold",
                                                    v,
                                                )
                                        }
                                    />
                                </Field.Field>

                                <Field.Field>
                                    <Field.Label for="delay_minor_mins"
                                        >Delay for minor posts (mins)</Field.Label
                                    >
                                    <Field.Description
                                        >How long after a 'minor' post is
                                        resolved should it be unstickied</Field.Description
                                    >

                                    <Input
                                        id="delay_minor_mins"
                                        type="number"
                                        bind:value={
                                            () =>
                                                current.sticky
                                                    ?.delay_minor_mins ?? 0,
                                            (v) =>
                                                _.set(
                                                    pending,
                                                    "sticky.delay_minor_mins",
                                                    v,
                                                )
                                        }
                                    />
                                </Field.Field>

                                <Field.Field>
                                    <Field.Label for="delay_major_mins"
                                        >Delay for major posts (mins)</Field.Label
                                    >
                                    <Field.Description
                                        >How long after a 'major' post is
                                        resolved should it be unstickied</Field.Description
                                    >

                                    <Input
                                        id="delay_major_mins"
                                        type="number"
                                        bind:value={
                                            () =>
                                                current.sticky
                                                    ?.delay_major_mins ?? 0,
                                            (v) => {
                                                _.set(
                                                    pending,
                                                    "sticky.delay_major_mins",
                                                    v,
                                                );
                                            }
                                        }
                                    />
                                </Field.Field>
                            {/if}
                        </Field.Set>
                    </Field.Group>
                {/snippet}
            </Module>

            <Module
                key="staff_reply"
                title="Collect staff replies in a stickied comment"
                {options}
                {open}
                bind:changes
            >
                {#snippet children({ current, pending })}
                    <div class="w-full max-w-md">
                        <Field.Set>
                            <Field.Group>
                                <Field.Field>
                                    <Field.Label for="flair_id"
                                        >Flair ID</Field.Label
                                    >
                                    <Field.Description
                                        >Users with a flair with this ID are
                                        considered staff</Field.Description
                                    >
                                    <Input
                                        id="flair_id"
                                        type="text"
                                        placeholder="28ccdbf7-1992-425b-8e8f-0d5ab9b2d4ad"
                                        bind:value={
                                            () => current.flair_id,
                                            (v) => (pending.flair_id = v)
                                        }
                                    />
                                </Field.Field>
                                <Field.Field>
                                    <Field.Label for="css_class"
                                        >CSS Class</Field.Label
                                    >
                                    <Field.Description
                                        >Users with a flair with this CSS class
                                        are considered staff</Field.Description
                                    >
                                    <InputClearable
                                        id="css_class"
                                        type="text"
                                        placeholder="staff"
                                        bind:value={
                                            () => current.css_class,
                                            (v) => (pending.css_class = v)
                                        }
                                    />
                                </Field.Field>
                            </Field.Group>
                        </Field.Set>
                    </div>
                {/snippet}
            </Module>

            <Module
                key="ai_slop"
                title="Scan repository links for AI slop"
                {options}
                {open}
                bind:changes
            />

            <Module
                key="related_title"
                title="Remove posts with vague titles"
                {options}
                {open}
                bind:changes
            />

            <Module
                key="complex_comments"
                title="Remove comments based on post contents"
                {options}
                {open}
                bind:changes
            />

            <Module
                key="comments_code"
                title="Convert three-backtick code blocks to four-spaces"
                {options}
                {open}
                bind:changes
            />

            <Module
                key="comments_cdn"
                title="Warn users about posting temporary CDN links"
                {options}
                {open}
                bind:changes
            />
        </Accordion.Root>
    {/if}
</main>
