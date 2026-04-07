<script lang="ts">
    import { Badge } from "../ui/badge";
    import { TableCell } from "../ui/table";

    interface InfoProperties {
        enabled: boolean;
        report: boolean;
        remove: boolean;

        ocr?: unknown;
        title?: unknown;
        body?: unknown;
        title_or_body?: unknown;
    }

    interface Props {
        scam: InfoProperties;
    }

    let { scam }: Props = $props();
</script>

<TableCell>
    <div class="float-start">
        {#if !scam.enabled}
            <Badge variant="outline">Disabled</Badge>
        {/if}

        {#if scam.report && scam.remove}
            <Badge variant="secondary">Filter</Badge>
        {:else if scam.report}
            <Badge>Report</Badge>
        {:else if scam.remove}
            <Badge variant="destructive">Remove</Badge>
        {:else}
            <!-- no action -->
        {/if}
    </div>

    <div class="float-end">
        {#if scam.ocr}
            <Badge>OCR</Badge>
        {/if}

        {#if scam.title ?? scam.title_or_body}
            <Badge>Title</Badge>
        {/if}

        {#if scam.body ?? scam.title_or_body}
            <Badge>Body</Badge>
        {/if}
    </div>
</TableCell>
