<script lang="ts">
    import "./layout.css";
    import { Toaster } from "$lib/components/ui/sonner/index.js";

    import favicon from "$lib/assets/favicon.svg";
    import type { LayoutProps } from "./$types";
    import * as Sidebar from "$lib/components/ui/sidebar";
    import * as Collapsible from "$lib/components/ui/collapsible";
    import { ChevronDown } from "@lucide/svelte";

    let { children, data }: LayoutProps = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<Toaster />

<Sidebar.Provider>
    <Sidebar.Root>
        <Sidebar.Header>/u/mlapibot</Sidebar.Header>
        <Sidebar.Content>
            <Sidebar.Group>
                <Sidebar.GroupContent>
                    <Sidebar.Menu>
                        {#if data.me !== undefined}
                            <Collapsible.Root class="group/collapsible">
                                <Sidebar.MenuItem>
                                    <Collapsible.Trigger>
                                        {#snippet child({ props })}
                                            <Sidebar.MenuButton {...props}>
                                                Subreddits

                                                <ChevronDown
                                                    class="ms-auto transition-transform group-data-[state=open]/collapsible:rotate-180"
                                                />
                                            </Sidebar.MenuButton>
                                        {/snippet}
                                    </Collapsible.Trigger>
                                    <Collapsible.Content>
                                        <Sidebar.MenuSub>
                                            {#each data.subs as sub}
                                                <Sidebar.MenuSubItem>
                                                    <Sidebar.SidebarMenuSubButton
                                                    >
                                                        {#snippet child({
                                                            props,
                                                        })}
                                                            <a
                                                                href={`/subreddits/${sub.name}`}
                                                                {...props}
                                                            >
                                                                /r/{sub.name}
                                                            </a>
                                                        {/snippet}
                                                    </Sidebar.SidebarMenuSubButton>
                                                </Sidebar.MenuSubItem>
                                            {/each}
                                        </Sidebar.MenuSub>
                                    </Collapsible.Content>
                                </Sidebar.MenuItem>
                            </Collapsible.Root>

                            {#if data.me.admin}
                                <Sidebar.MenuItem>
                                    <Sidebar.MenuButton>
                                        {#snippet child({ props })}
                                            <a href="/settings" {...props}>
                                                Shhhhh
                                            </a>
                                        {/snippet}
                                    </Sidebar.MenuButton>
                                </Sidebar.MenuItem>
                            {/if}

                            <Sidebar.MenuItem>
                                <Sidebar.MenuButton>
                                    {#snippet child({ props })}
                                        <a href="/auth" {...props}>
                                            /u/{data.me.name}
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
                    </Sidebar.Menu>
                </Sidebar.GroupContent>
            </Sidebar.Group>
        </Sidebar.Content>
    </Sidebar.Root>

    <Sidebar.Trigger />
    <main class="w-full py-1 px-2">
        {@render children()}
    </main>
</Sidebar.Provider>
