<script lang="ts">
    import type { WithChildren } from "bits-ui";
    import { Badge } from "../badge";
    import { Spinner } from ".";

    interface Props extends WithChildren {}

    let { children }: Props = $props();
</script>

<Badge>
    <Spinner />
    {@render children?.()}
</Badge>
