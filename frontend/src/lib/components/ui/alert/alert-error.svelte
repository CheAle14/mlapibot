<script lang="ts">
    import Alert from "./alert.svelte";
    import AlertTitle from "./alert-title.svelte";
    import AlertDescription from "./alert-description.svelte";
    import { isHttpError } from "@sveltejs/kit";
    import { CircleAlert } from "@lucide/svelte";

    interface Props {
        title: string;
        error: unknown;
    }

    let { title, error }: Props = $props();
</script>

<Alert variant="destructive">
    <CircleAlert />

    <AlertTitle>{title}</AlertTitle>

    <AlertDescription>
        {#if isHttpError(error)}
            {error.body.message}
        {:else}
            Unknown error occurred
        {/if}
    </AlertDescription>
</Alert>
