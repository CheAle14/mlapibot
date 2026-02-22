<script lang="ts">
    import * as _ from "moderndash";
    import type { PageProps } from "./$types";
    import * as Accordion from "$lib/components/ui/accordion";
    import * as Field from "$lib/components/ui/field";
    import { Input } from "$lib/components/ui/input";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { Switch } from "$lib/components/ui/switch";
    import {
        type PendingSubredditOptions,
        type SubredditOptions,
    } from "$lib/types/subreddit";
    import { Json } from "$lib/components/ui/json";
    import Module from "./Module.svelte";
    import { SelectIncidentImpact } from "$lib/components/reuse/select";
    import { Button } from "$lib/components/ui/button";

    const { params, data }: PageProps = $props();

    function defaultPending(): PendingSubredditOptions {
        return {
            ai_slop: {},
            staff_reply: {},
            status: {},
            scams: {},
            related_title: {},
        };
    }

    let changes = $state<PendingSubredditOptions>(defaultPending());

    const hasChanges = $derived.by(() => {
        for (const key of Object.keys(changes)) {
            const value = changes[key as keyof PendingSubredditOptions];
            if (!_.isEqual(value, {})) {
                return true;
            }
        }

        return false;
    });

    const toggleSticky = (v: boolean) => {
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
        for (const key of Object.keys(changes)) {
            changes[key as keyof PendingSubredditOptions].enabled = false;
        }
    };

    let options = $derived(_.merge(data.subdata, changes) as SubredditOptions);
</script>

<h2>/r/{params.subreddit}</h2>

<main class="w-11/12">
    <div class="flex flex-row justify-around">
        <Button disabled={!hasChanges}>Save changes</Button>

        <Button disabled={hasChanges} variant="destructive" onclick={disableAll}
            >DISABLE ALL</Button
        >

        <Button
            disabled={!hasChanges}
            onclick={() => (changes = defaultPending())}>Revert changes</Button
        >
    </div>

    <Json value={changes} title="Pending Changes" />

    <Accordion.Root type="multiple" class="">
        <Module key="scams" title="OCR" {options} bind:changes />

        <Module
            key="status"
            title="Discord status incidents"
            {options}
            bind:changes
        >
            {#snippet children(current, pending)}
                <div class="w-full max-w-md">
                    <Field.Group>
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

                        <Field.Separator />

                        <Field.Set>
                            <Field.Field orientation="horizontal">
                                <Switch
                                    id="sticky"
                                    bind:checked={
                                        () => current.sticky !== undefined,
                                        toggleSticky
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
                                            (v) =>
                                                _.set(
                                                    pending,
                                                    "sticky.delay_major_mins",
                                                    v,
                                                )
                                        }
                                    />
                                </Field.Field>
                            {/if}
                        </Field.Set>
                    </Field.Group>
                </div>
            {/snippet}
        </Module>

        <Module
            key="ai_slop"
            title="Scan repository links for AI slop"
            {options}
            bind:changes
        />

        <Module
            key="related_title"
            title="Remove posts with vague titles"
            {options}
            bind:changes
        />

        <Module
            key="staff_reply"
            title="Collect staff replies in a stickied comment"
            {options}
            bind:changes
        >
            {#snippet children(current, pending)}
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
                                    >Users with a flair with this CSS class are
                                    considered staff</Field.Description
                                >
                                <Input
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
    </Accordion.Root>
</main>
