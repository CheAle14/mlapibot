<script lang="ts">
    import type { WithChildren } from "bits-ui";

    interface Props extends WithChildren {
        id?: string;
        onsubmit: (e: Event) => void;
    }

    let { onsubmit, children, ...formProps }: Props = $props();

    const handleSubmit = (e: Event) => {
        e.preventDefault();
        onsubmit(e);
        return false;
    };
</script>

<form onsubmit={handleSubmit} {...formProps}>
    {@render children?.()}
</form>
