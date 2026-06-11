<script lang="ts">
    import type { HTMLAnchorAttributes } from "svelte/elements";

    let { children, ...rest }: HTMLAnchorAttributes = $props();
</script>

<a class="text-blue-700 underline" {...rest}>{@render children?.()}</a>
