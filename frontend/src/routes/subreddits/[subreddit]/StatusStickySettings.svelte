<script lang="ts">
    import { SelectIncidentImpact } from "$lib/components/reuse/select";
    import * as Field from "$lib/components/ui/field";
    import { Input, InputClearable, InputList } from "$lib/components/ui/input";
    import type { StatusStickyConfig } from "$lib/types/subreddit";

    interface Updater {
        <K extends keyof StatusStickyConfig>(
            key: K,
            value: StatusStickyConfig[K],
        ): void;
    }

    interface Props {
        status_url: string;
        current: StatusStickyConfig;
        update: Updater;
    }

    let { status_url, current, update }: Props = $props();

    let components_link = $derived(status_url + "/components.json");
</script>

<Field.Field>
    <Field.Label for="sticky.min_impact">Minimum Impact</Field.Label>
    <Field.Description
        >Incidents below this impact will not be stickied</Field.Description
    >

    <SelectIncidentImpact
        bind:value={() => current.min_impact, (v) => update("min_impact", v)}
    />
</Field.Field>

<Field.Field>
    <Field.Label for="replace_sticky">Replace Sticky</Field.Label>
    <Field.Description
        >If a post with this flair ID is already stickied, replace it. Once the
        incident is over (as determined below) it will be re-stickied.</Field.Description
    >

    <InputClearable
        bind:value={
            () => current.replace_sticky, (v) => update("replace_sticky", v)
        }
        placeholder="some-flair-id-here"
    />
</Field.Field>

<Field.Field>
    <Field.Label for="comment_threshold">Comment threshold</Field.Label>
    <Field.Description
        >The threshold that determines whether a post is 'minor' or 'major', for
        the two following settings</Field.Description
    >

    <Input
        type="number"
        bind:value={
            () => current.comment_threshold,
            (v) => update("comment_threshold", v)
        }
    />
</Field.Field>

<Field.Field>
    <Field.Label for="delay_minor_mins"
        >Delay for minor posts (mins)</Field.Label
    >
    <Field.Description
        >How long after a 'minor' post is resolved should it be unstickied</Field.Description
    >

    <Input
        id="delay_minor_mins"
        type="number"
        bind:value={
            () => current.delay_minor_mins, (v) => update("delay_minor_mins", v)
        }
    />
</Field.Field>

<Field.Field>
    <Field.Label for="delay_major_mins"
        >Delay for major posts (mins)</Field.Label
    >
    <Field.Description
        >How long after a 'major' post is resolved should it be unstickied</Field.Description
    >

    <Input
        id="delay_major_mins"
        type="number"
        bind:value={
            () => current.delay_major_mins, (v) => update("delay_major_mins", v)
        }
    />
</Field.Field>

<Field.Field>
    <Field.Label>Component filter</Field.Label>
    <Field.Description
        ><p>Only sticky posts whose component ID or name is in this list.</p>
        <p>
            You should be able to find the IDs in <a href={components_link}
                >{components_link}</a
            >
        </p></Field.Description
    >

    <InputList
        noedit
        type="text"
        popoverClass="lg:w-md"
        bind:value={
            () => current.only_for,
            (v) => {
                console.log("yeet", v);
                update("only_for", v);
            }
        }
    />
</Field.Field>
