<script lang="ts">
    import { fetchSubPosts } from "$lib/api/posts.remote";
    import { Button } from "$lib/components/ui/button";
    import { Json } from "$lib/components/ui/json";
    import Spinner from "$lib/components/ui/spinner/spinner.svelte";
    import * as Table from "$lib/components/ui/table";
    import type { PageProps } from "./$types";

    const { params, data }: PageProps = $props();

    const sub = $derived(data.subs.find((s) => s.name === params.subreddit));

    const posts = $derived(fetchSubPosts(sub?.id ?? "<>"));

    $inspect(posts.current);
</script>

{#if posts.current === undefined}
    <Spinner /> Fetching
{:else}
    <Table.Root>
        <Table.Header>
            <Table.Row>
                <Table.Head>Title</Table.Head>
                <Table.Head>Status</Table.Head>
                <Table.Head>Actions</Table.Head>
            </Table.Row>
        </Table.Header>

        <Table.Body>
            {#each posts.current as post (post.id)}
                <Table.Row>
                    <Table.Cell>
                        {post.title}
                    </Table.Cell>
                    <Table.Cell>
                        {#if post.reddit_id}
                            <a
                                class="underline underline-offset-4"
                                href={`https://reddit.com/r/${params.subreddit}/comments/${post.reddit_id}`}
                                >Published</a
                            >
                        {:else}
                            Draft
                        {/if}
                    </Table.Cell>
                    <Table.Cell>
                        <Button
                            href={`/subreddits/${params.subreddit}/posts/${post.id}`}
                            >Edit</Button
                        >
                    </Table.Cell>
                </Table.Row>
            {/each}
        </Table.Body>

        <Table.Footer>
            <Table.Row>
                <Table.Cell colspan={3}>
                    <Button
                        class="float-right"
                        href={`/subreddits/${params.subreddit}/posts/new`}
                        >New</Button
                    >
                </Table.Cell>
            </Table.Row>
        </Table.Footer>
    </Table.Root>
{/if}
