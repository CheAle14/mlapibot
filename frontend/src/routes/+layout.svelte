<script lang="ts">
    import "./layout.css";
    import { Toaster } from "$lib/components/ui/sonner/index.js";

    import favicon from "$lib/assets/favicon.svg";
    import type { LayoutProps } from "./$types";
    import * as Sidebar from "$lib/components/ui/sidebar";
    import * as Collapsible from "$lib/components/ui/collapsible";
    import { ChevronDown, Shield } from "@lucide/svelte";
    import { page } from "$app/state";

    let { children, data }: LayoutProps = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<Toaster richColors />

<Sidebar.Provider>
    <Sidebar.Root>
        <Sidebar.Header>/u/mlapibot</Sidebar.Header>
        <Sidebar.Content>
            <Sidebar.Group>
                {#if data.me !== undefined}
                    {#if data.me.admin}
                        <Sidebar.MenuItem>
                            <Sidebar.MenuButton>
                                {#snippet child({ props })}
                                    <a href="/settings" {...props}> Shhhhh </a>
                                {/snippet}
                            </Sidebar.MenuButton>
                        </Sidebar.MenuItem>
                    {/if}

                    <Sidebar.MenuItem>
                        <Sidebar.MenuButton>
                            {#snippet child({ props })}
                                <a href="/auth" {...props}>
                                    /u/{data?.me?.name ?? "<never undefined>"}
                                </a>
                            {/snippet}
                        </Sidebar.MenuButton>
                    </Sidebar.MenuItem>
                {:else}
                    <Sidebar.MenuItem>
                        <Sidebar.MenuButton>
                            {#snippet child({ props })}
                                <a href="/auth" {...props}> Login </a>
                            {/snippet}
                        </Sidebar.MenuButton>
                    </Sidebar.MenuItem>
                {/if}
            </Sidebar.Group>

            <Sidebar.Group>
                <Sidebar.GroupLabel>Subreddits</Sidebar.GroupLabel>

                {#each data.subs as sub (sub.id)}
                    {@const href = "/subreddits/" + sub.name}
                    <Collapsible.Root class="group/collapsible">
                        <Sidebar.MenuItem>
                            <Collapsible.Trigger>
                                {#snippet child({ props })}
                                    <Sidebar.MenuButton {...props}>
                                        /r/{sub.name}

                                        {#if data.me?.admin && sub.is_mod}
                                            <Shield class="right-0 absolute" />
                                        {/if}

                                        <ChevronDown
                                            class="ms-auto transition-transform group-data-[state=open]/collapsible:rotate-180"
                                        />
                                    </Sidebar.MenuButton>
                                {/snippet}
                            </Collapsible.Trigger>
                            <Collapsible.Content>
                                <Sidebar.MenuSub>
                                    <Sidebar.MenuSubItem>
                                        <Sidebar.MenuSubButton>
                                            {#snippet child({ props })}
                                                <a {href} {...props}>
                                                    Settings
                                                </a>
                                            {/snippet}
                                        </Sidebar.MenuSubButton>
                                    </Sidebar.MenuSubItem>

                                    <Sidebar.MenuSubItem>
                                        <Sidebar.MenuSubButton>
                                            {#snippet child({ props })}
                                                <a
                                                    href={href + "/posts"}
                                                    {...props}
                                                >
                                                    Posts
                                                </a>
                                            {/snippet}
                                        </Sidebar.MenuSubButton>
                                    </Sidebar.MenuSubItem>

                                    <Sidebar.MenuSubItem>
                                        <Sidebar.MenuSubButton>
                                            {#snippet child({ props })}
                                                <a
                                                    href={href +
                                                        "/staff_replies"}
                                                    {...props}
                                                >
                                                    Staff Replies
                                                </a>
                                            {/snippet}
                                        </Sidebar.MenuSubButton>
                                    </Sidebar.MenuSubItem>
                                </Sidebar.MenuSub>
                            </Collapsible.Content>
                        </Sidebar.MenuItem>
                    </Collapsible.Root>
                {/each}
            </Sidebar.Group>
        </Sidebar.Content>
    </Sidebar.Root>

    <Sidebar.Inset>
        <Sidebar.Trigger />

        <div class="px-2 py-1">
            {@render children()}
        </div>
    </Sidebar.Inset>
</Sidebar.Provider>
