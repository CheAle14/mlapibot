<script lang="ts">
    import { fetchStaffRepliesInThread } from "$lib/api/staff_replies.remote";
    import { Anchor } from "$lib/components/reuse/anchor";
    import { PagedTable } from "$lib/components/reuse/paged-table";
    import { TableRow, TableCell, TableHead } from "$lib/components/ui/table";
    import type { PageReq } from "$lib/types/pagination";
    import type { StaffReply } from "$lib/types/staff_replies";
    import * as Item from "$lib/components/ui/item";
    import Markdown from "svelte-exmarkdown";
    import { Skeleton } from "$lib/components/ui/skeleton";

    interface Props {
        subreddit_name: string;
        subreddit_id: string;
        post_id: string;
    }

    let { subreddit_name, subreddit_id, post_id }: Props = $props();

    let page = $state<PageReq>({
        page: 0,
        limit: 100,
    });

    let query = $derived(
        fetchStaffRepliesInThread({
            subreddit_id,
            post_id,
            page,
        }),
    );
</script>

<Item.Root variant="outline" class="w-auto m-2">
    <Item.Content>
        <PagedTable {query} bind:page key={(i) => i.comment_id}>
            {#snippet header()}
                <TableRow>
                    <TableHead>Link</TableHead>
                    <TableHead>Author</TableHead>
                    <TableHead>Content</TableHead>
                </TableRow>
            {/snippet}

            {#snippet skeletonRow()}
                <TableRow>
                    <TableCell>
                        <Skeleton class="h-4 w-15" />
                    </TableCell>
                    <TableCell>
                        <Skeleton class="h-4 w-30" />
                    </TableCell>
                    <TableCell>
                        <Skeleton class="h-4 w-100" />
                    </TableCell>
                </TableRow>
            {/snippet}

            {#snippet row(item: StaffReply)}
                <TableRow>
                    <TableCell>
                        <Anchor
                            href={`https://reddit.com/r/${subreddit_name}/comments/${post_id}/-/${item.comment_id}`}
                        >
                            {item.comment_id}
                        </Anchor>
                    </TableCell>
                    <TableCell>
                        <Anchor
                            href={`https://reddit.com/u/${item.author_name}`}
                        >
                            /u/{item.author_name}
                        </Anchor>
                    </TableCell>
                    <TableCell>
                        <Markdown md={item.content} />
                    </TableCell>
                </TableRow>
            {/snippet}
        </PagedTable>
    </Item.Content>
</Item.Root>
