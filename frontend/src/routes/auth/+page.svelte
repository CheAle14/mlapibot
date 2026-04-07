<script lang="ts">
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

{#if data.me}
    <p>
        You are currently logged in as <strong>/u/{data.me.name}</strong> with
        user ID {data.me.id}
    </p>

    <p>To logout, clear your cookies.</p>
{/if}
<a class="text-blue-700 underline" href={data.redirectUrl}>Login to Reddit</a>
