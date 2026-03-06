<script lang="ts">
    import { getTemplateStubs } from "$lib/api/templates.remote";
    import type { CreateOrStubTemplateInfo } from "$lib/types/templates";
    import * as MySelect from "$lib/components/reuse/select";
    import type { SubredditTemplateStub } from "$lib/types/subreddit";

    interface Props {
        subreddit: string;
        deleted_templates?: number[];

        value?: number;
    }

    let { value = $bindable(), subreddit, deleted_templates }: Props = $props();
    let templates = $derived(getTemplateStubs(subreddit));

    const options = $derived.by(() => {
        const items: SubredditTemplateStub[] = [];

        for (const item of templates.current ?? []) {
            if (deleted_templates && deleted_templates.indexOf(item.id) !== -1)
                continue;

            items.push(item);
        }

        return items;
    });
</script>

<MySelect.Simple
    clearable
    {options}
    placeholder={templates.loading ? "Loading ..." : undefined}
    bind:selected={
        () => options.find((i) => i.id === value),
        (v) => {
            if (!templates.loading) {
                value = v?.id;
            }
        }
    }
>
    {#snippet trigger(v)}
        {v.name}
    {/snippet}

    {#snippet item(v)}
        <strong>{v.id}</strong>: {v.name}
    {/snippet}
</MySelect.Simple>
