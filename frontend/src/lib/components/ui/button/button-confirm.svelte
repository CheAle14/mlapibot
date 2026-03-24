<script lang="ts">
    import type { WithChildren } from "bits-ui";
    import * as Popover from "../popover";
    import {
        Button,
        buttonVariants,
        type ButtonProps,
    } from "$lib/components/ui/button";
    import type { Snippet } from "svelte";

    interface Props extends WithChildren, ButtonProps {
        onclick(): void;
        description: string;
        modal?: Snippet<[]>;
    }

    let { onclick, children, modal, description, ...rest }: Props = $props();

    let open = $state(false);

    const handleConfirmed = () => {
        open = false;
        onclick();
    };
</script>

<Popover.Root bind:open>
    <Popover.Trigger>
        {#snippet child({ props })}
            <Button {...rest} {...props}>
                {@render children?.()}
            </Button>
        {/snippet}
    </Popover.Trigger>

    <Popover.Content class="w-auto">
        {#if modal}
            {@render modal()}
        {:else}
            <div class="flex flex-col items-center w-3xs">
                <h1>Are you sure?</h1>

                <p>{description}</p>

                <Button
                    size="sm"
                    variant="destructive"
                    onclick={handleConfirmed}>Confirm</Button
                >
            </div>
        {/if}
    </Popover.Content>
</Popover.Root>
