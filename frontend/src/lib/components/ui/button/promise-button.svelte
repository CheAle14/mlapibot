<script lang="ts">
    import type { WithChildren } from "bits-ui";
    import * as Popover from "../popover";
    import {
        Button,
        buttonVariants,
        type ButtonProps,
    } from "$lib/components/ui/button";
    import Spinner from "../spinner/spinner.svelte";
    import { success } from "zod/mini";
    import { Check, X } from "@lucide/svelte";

    interface Props extends WithChildren, ButtonProps {
        onclick(): Promise<unknown>;
        successTickTimeMs?: number;
        errorTickTimeMs?: number;
    }

    let {
        onclick,
        successTickTimeMs = 1000,
        errorTickTimeMs,
        size,
        disabled,
        children,
        ...rest
    }: Props = $props();

    let promiseState = $state<"idle" | "loading" | "success" | "error">("idle");

    const handleClick = async () => {
        promiseState = "loading";
        try {
            await onclick();

            if (successTickTimeMs) {
                promiseState = "success";
                setTimeout(() => (promiseState = "idle"), successTickTimeMs);
            } else {
                promiseState = "idle";
            }
        } catch (err) {
            console.error("button failed:", err);
            promiseState = "error";
            if (errorTickTimeMs) {
                setTimeout(() => (promiseState = "idle"), errorTickTimeMs);
            }
        }
    };

    const replaceChildren = $derived(size?.includes("icon") || false);
</script>

<Button
    onclick={handleClick}
    disabled={disabled || promiseState === "loading"}
    {size}
    {...rest}
>
    {#if promiseState !== "idle"}
        {#if promiseState === "loading"}
            <Spinner />
        {:else if promiseState === "success"}
            <Check class="text-green-500" />
        {:else}
            <X class="text-red-500" />
        {/if}

        {#if !replaceChildren}
            {@render children?.()}
        {/if}
    {:else}
        {@render children?.()}
    {/if}
</Button>
