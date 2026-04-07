<script lang="ts">
    import type { Snippet } from "svelte";
    import { Field, FieldLabel, FieldDescription } from "../ui/field";
    import { Textarea } from "../ui/textarea";
    import { Button } from "../ui/button";
    import { Item, ItemContent } from "../ui/item";
    import Markdown from "svelte-exmarkdown";

    interface Props {
        content: string;
        disabled?: boolean;
        buttons: Snippet<[]>;
    }

    let { content = $bindable(), buttons, ...rest }: Props = $props();
</script>

<Field>
    <FieldLabel>Content ({content.length} / 10000)</FieldLabel>

    <FieldDescription
        >Unfortunately the preview does not support some syntax such as <code
            >^supertext</code
        >
        or tables</FieldDescription
    >

    <div class="flex flex-col xl:flex-row gap-2 pr-5">
        <div class="flex-1/2 max-w-2xl">
            <Textarea required rows={12} bind:value={content} {...rest} />

            <div class="mt-2 w-full flex flex-col gap-2">
                {@render buttons()}
            </div>
        </div>

        <Item variant="muted" class="flex-1/2 wrap-anywhere max-w-3xl">
            <ItemContent class="markdown">
                <Markdown md={content} />
            </ItemContent>
        </Item>
    </div>
</Field>
