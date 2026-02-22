<script lang="ts">
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

<a href={data.redirectUrl}>Login to Reddit</a>
