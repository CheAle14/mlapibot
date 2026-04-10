<script lang="ts">
    import * as _ from "moderndash";
    import type { PageProps } from "./$types";
    import * as Accordion from "$lib/components/ui/accordion";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputClearable, InputList } from "$lib/components/ui/input";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { Switch } from "$lib/components/ui/switch";
    import { ModuleKeys, type ApiSubredditOptions } from "$lib/types/subreddit";
    import { Json, JsonMany } from "$lib/components/ui/json";
    import Module from "./Module.svelte";
    import {
        RemovalReason,
        SelectIncidentImpact,
    } from "$lib/components/reuse/select";
    import { Button } from "$lib/components/ui/button";
    import ScamsTable from "./ScamsTable.svelte";
    import * as Spinner from "$lib/components/ui/spinner";
    import RemovalReasons from "./RemovalReasons.svelte";
    import Templates from "$lib/components/templates/Templates.svelte";
    import { getSubredditOptions } from "$lib/api/options.remote";
    import { savePendingChanges } from "$lib/api/changes.remote";
    import StatusStickySettings from "./StatusStickySettings.svelte";
    import { Check } from "@lucide/svelte";
    import ComplexCommentsSettings from "./ComplexCommentsSettings.svelte";
    import { beforeNavigate } from "$app/navigation";

    const STATUS_URL = "https://discordstatus.com/api/v2";
    const { params, data }: PageProps = $props();

    const subreddit = $derived(
        data.subs.find((s) => s.name === params.subreddit)?.id ?? "???",
    );

    const {
        loading: isFetching,
        current: options,
        error,
    } = $derived(getSubredditOptions(subreddit));

    let open: string[] = $state([]);

    let changes = $state<ApiSubredditOptions | undefined>();

    $effect(() => {
        changes = options;
    });

    const revertPendingChanges = () => {
        changes = options;
    };

    let sendPendingState = $state<"idle" | "sending" | "error" | "success">(
        "idle",
    );

    const sendPendingChanges = async () => {
        sendPendingState = "sending";
        try {
            if (changes) {
                await savePendingChanges({ subreddit, changes });
                sendPendingState = "success";
                await _.sleep(2000);
                sendPendingState = "idle";
            }
        } catch (err) {
            sendPendingState = "error";
            console.error(err);
        }
    };

    const hasChanges = $derived.by(() => {
        const oAny = (options ?? {}) as any;
        const cAny = (changes ?? {}) as any;

        const allkeys = new Set([...Object.keys(oAny), ...Object.keys(cAny)]);

        allkeys.delete("seq_num");

        for (const key of allkeys) {
            if (!_.isEqual(oAny[key], cAny[key])) {
                console.log(key, oAny[key], cAny[key]);
                return true;
            }
        }

        return options !== undefined && !_.isEqual(options, changes);
    });

    beforeNavigate((nav) => {
        if (hasChanges) {
            if (nav.to?.route.id) {
                // client-side routing
                const confirmed = confirm(
                    "You have unsaved changes. Are you sure you wish to leave?",
                );

                if (!confirmed) {
                    nav.cancel();
                }
            } else {
                // external nav uses the browser's built-in prompt
                nav.cancel();
            }
        }
    });

    const isAllDisabled = $derived.by(() => {
        for (const key of ModuleKeys) {
            if (changes && changes[key].enabled) {
                return false;
            }
        }
        return true;
    });

    const toggleSticky = (v: boolean) => {
        if (!changes) return;
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
        if (!changes) return;

        for (const key of ModuleKeys) {
            changes[key].enabled = false;
        }
    };
</script>

<h2>
    /r/{params.subreddit} (t5_{subreddit}) {#if options}[seq-{options.seq_num}]{/if}
</h2>

<main class="w-11/12">
    <div class="flex flex-row justify-around">
        <Button
            disabled={!hasChanges}
            pending={sendPendingState === "sending"}
            errored={sendPendingState === "error"}
            onclick={sendPendingChanges}
        >
            {#if sendPendingState === "success"}
                <Check color="green" />
            {/if}

            Save changes
        </Button>

        <Button
            disabled={isAllDisabled}
            variant="destructive"
            onclick={disableAll}>DISABLE ALL</Button
        >

        <Button disabled={!hasChanges} onclick={revertPendingChanges}
            >Revert changes</Button
        >
    </div>

    {#if isFetching}
        <Spinner.Badge>Fetching subreddit options</Spinner.Badge>
    {/if}

    {#if error}
        <Json value={error} title="Error fetching data" />
    {/if}

    {#if options && changes}
        <Accordion.Root type="multiple" class="" bind:value={open}>
            <Accordion.Item value="removal_reasons">
                <Accordion.Trigger>Removal reasons map</Accordion.Trigger>

                <Accordion.Content class="pl-2">
                    <p>
                        These aliases are primarily to allow OCR rules to be
                        copy-pasted between a test subreddit and this one
                    </p>
                    <RemovalReasons
                        original={options.removal_reasons}
                        bind:current={changes.removal_reasons}
                    />
                </Accordion.Content>
            </Accordion.Item>

            <Accordion.Item value="templates">
                <Accordion.Trigger>Post reply templates</Accordion.Trigger>

                <Accordion.Content class="pl-2">
                    {#if open.indexOf("templates") !== -1}
                        <Templates
                            {subreddit}
                            bind:updates={changes.templates.updates}
                            bind:creates={changes.templates.creates}
                            bind:deletes={changes.templates.deletes}
                        />
                    {/if}
                </Accordion.Content>
            </Accordion.Item>

            <Module
                key="scams"
                title="Remove posts based on OCR or text content"
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ open, current })}
                    {#if open}
                        <ScamsTable
                            removal_reasons={changes?.removal_reasons ?? {}}
                            deleted_templates={changes?.templates.deletes}
                            updates={current.update}
                            creates={current.create}
                            deletes={current.deletes}
                            subreddit_id={subreddit}
                        />
                    {/if}

                    <div class="w-full max-w-md">
                        <Field.Set>
                            <Field.Legend>Settings</Field.Legend>
                            <Field.Group>
                                <Field.Field orientation="horizontal">
                                    <Checkbox
                                        id="search_modqueue"
                                        bind:checked={
                                            () =>
                                                current.search_modqueue ??
                                                false,
                                            (v) => (current.search_modqueue = v)
                                        }
                                    />

                                    <Field.Content>
                                        <Field.Label for="search_modqueue"
                                            >Search Modqueue</Field.Label
                                        >
                                        <Field.Description
                                            >If checked, also apply rules
                                            against the subreddit's modqueue</Field.Description
                                        >
                                    </Field.Content>
                                </Field.Field>
                            </Field.Group>
                        </Field.Set>
                    </div>
                {/snippet}
            </Module>

            <Module
                key="status"
                title="Discord status incidents"
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ current })}
                    <Field.Group class="flex flex-col lg:flex-row">
                        <Field.Set>
                            <Field.Legend>Incident post</Field.Legend>
                            <Field.Description
                                >Settings for when and how an incident post is
                                made</Field.Description
                            >

                            <Field.Field>
                                <Field.Label for="api_url"
                                    >Status API URL</Field.Label
                                >
                                <Field.Description
                                    >The URL where the Atlassian StatusPage API
                                    is located</Field.Description
                                >

                                <Input
                                    type="text"
                                    disabled
                                    value={STATUS_URL}
                                />
                            </Field.Field>

                            <Field.Field>
                                <Field.Label for="min_impact"
                                    >Minimum Impact</Field.Label
                                >
                                <Field.Description
                                    >Incidents below this impact will not be
                                    posted</Field.Description
                                >

                                <SelectIncidentImpact
                                    bind:value={current.min_impact}
                                />
                            </Field.Field>
                            <Field.Field orientation="horizontal">
                                <Checkbox
                                    id="distinguish"
                                    bind:checked={current.distinguish}
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

                            <Field.Field>
                                <Field.Label for="post_flair_id"
                                    >Post Flair ID</Field.Label
                                >
                                <Field.Description
                                    >If present, incidents posted to the
                                    subreddit will use this flair template.</Field.Description
                                >

                                <InputClearable
                                    id="post_flair_id"
                                    type="text"
                                    bind:value={current.flair_id}
                                    placeholder="some-flair-id-here"
                                />
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
                                <StatusStickySettings
                                    status_url={STATUS_URL}
                                    bind:current={current.sticky}
                                />
                            {/if}
                        </Field.Set>
                    </Field.Group>
                {/snippet}
            </Module>

            <Module
                key="staff_reply"
                title="Collect staff replies in a stickied comment"
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ current })}
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
                                        bind:value={current.flair_id}
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
                                        bind:value={current.css_class}
                                    />
                                </Field.Field>
                                <Field.Field>
                                    <Field.Label
                                        >Post title exclusion</Field.Label
                                    >
                                    <Field.Description
                                        >Ignore posts whose title contains this
                                        text</Field.Description
                                    >

                                    <InputList
                                        type="text"
                                        thingName="phrases"
                                        popoverClass="lg:w-md"
                                        bind:value={
                                            current.ignore_post_title_contains
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
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ open, current })}
                    <div class="w-full max-w-md">
                        <Field.Group>
                            <Field.Set>
                                <Field.Field>
                                    <Field.Label>Modmail recipient</Field.Label>
                                    <Field.Description
                                        ><p>
                                            Slop reports for /r/{params.subreddit}
                                            will be sent to this subreddit.
                                        </p>

                                        <p>
                                            If absent, send it to /r/{params.subreddit}
                                        </p></Field.Description
                                    >

                                    <InputClearable
                                        type="text"
                                        bind:value={current.modmail_to}
                                        placeholder={params.subreddit}
                                    />
                                </Field.Field>

                                <Field.Field orientation="horizontal">
                                    <Switch
                                        id="sticky"
                                        bind:checked={current.report}
                                    />

                                    <Field.Content>
                                        <Field.Label for="sticky"
                                            >Report</Field.Label
                                        >
                                        <Field.Description
                                            >Should potential AI slop be
                                            reported?</Field.Description
                                        >
                                    </Field.Content>
                                </Field.Field>
                            </Field.Set>
                        </Field.Group>
                    </div>
                {/snippet}
            </Module>

            <Module
                key="related_title"
                title="Remove posts with vague titles"
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ open, current })}
                    <div class="w-full max-w-md">
                        <Field.Group>
                            <Field.Set>
                                <Field.Field>
                                    <Field.Label>Removal reason</Field.Label>
                                    <Field.Description
                                        >The reason used to remove posts under
                                        this module</Field.Description
                                    >

                                    <RemovalReason
                                        required
                                        reasons={changes?.removal_reasons ?? {}}
                                        bind:value={current.reason}
                                    />
                                </Field.Field>

                                <Field.Field orientation="horizontal">
                                    <Checkbox
                                        bind:checked={current.check_img_posts}
                                    />

                                    <Field.Content>
                                        <Field.Label
                                            >Check Image Posts</Field.Label
                                        >
                                        <Field.Description
                                            >Whether image posts with vague
                                            titles should also be removed</Field.Description
                                        >
                                    </Field.Content>
                                </Field.Field>
                            </Field.Set>
                        </Field.Group>
                    </div>
                {/snippet}
            </Module>

            <Module
                key="complex_comments"
                title="Remove comments based on post contents"
                {open}
                bind:current={changes}
                original={options}
            >
                {#snippet children({ current, original })}
                    <ComplexCommentsSettings
                        removal_reasons={changes?.removal_reasons ?? {}}
                        bind:current={current.items}
                        original={original.items}
                    />
                {/snippet}
            </Module>

            <Module
                key="comments_code"
                title="Convert three-backtick code blocks to four-spaces"
                {open}
                bind:current={changes}
                original={options}
            />

            <Module
                key="comments_cdn"
                title="Warn users about posting temporary CDN links"
                {open}
                bind:current={changes}
                original={options}
            />
        </Accordion.Root>
    {/if}
</main>
