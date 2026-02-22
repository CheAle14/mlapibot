<script lang="ts">
    import type { PageProps } from "./$types";

    const { params, data }: PageProps = $props();
</script>

<h2>{params.subreddit}</h2>
