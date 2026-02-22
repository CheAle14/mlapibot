<script lang="ts">
    import * as Accordion from "$lib/components/ui/accordion";
    import { Switch } from "$lib/components/ui/switch";
    import { Label } from "$lib/components/ui/label";
    import type { SubredditOptions } from "$lib/types/subreddit";
    import type { WithChildren } from "bits-ui";

    interface ModuleProps extends WithChildren {
        key: keyof SubredditOptions;
        title: string;
        description?: string;
        enabled: boolean;
    }

    let { key, title, children, enabled = $bindable() }: ModuleProps = $props();
</script>

<Accordion.Item value={key}>
    <Accordion.Trigger>
        {#snippet outside()}
            <Switch
                id={key}
                checked={enabled}
                onclick={(e) => {
                    enabled = !enabled;
                    e.preventDefault();
                    e.stopPropagation();
                    return false;
                }}
            />
        {/snippet}
        <Label>{title}</Label>
    </Accordion.Trigger>

    <Accordion.Content class="pl-2">
        {#if children}
            {@render children()}
        {:else}
            There are no options for this module.
        {/if}
    </Accordion.Content>
</Accordion.Item>
