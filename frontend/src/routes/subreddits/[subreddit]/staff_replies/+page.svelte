<script lang="ts">
    import {
        fetchStaffReplyThreads,
        refreshStaffReplyThread,
    } from "$lib/api/staff_replies.remote";
    import { Anchor } from "$lib/components/reuse/anchor";
    import { PagedTable } from "$lib/components/reuse/paged-table";
    import { Json } from "$lib/components/ui/json";
    import { Spinner } from "$lib/components/ui/spinner";
    import { TableCell, TableHead, TableRow } from "$lib/components/ui/table";
    import type { PageReq, PageResp } from "$lib/types/pagination";
    import type { StaffReplyThread } from "$lib/types/staff_replies";
    import { RefreshCcw, Settings } from "@lucide/svelte";
    import type { PageProps } from "../$types";
    import { Button, PromiseButton } from "$lib/components/ui/button";
    import { toast } from "svelte-sonner";

    const { params, data }: PageProps = $props();

    let liveRefreshes = $state(0);

    let page = $state<PageReq>({
        page: 0,
        limit: 10,
    });

    const sub = $derived(data.subs.find((s) => s.name === params.subreddit));

    const threads = $derived(
        fetchStaffReplyThreads({
            subreddit_name: params.subreddit,
            subreddit_id: sub?.id ?? "",
            page,
        }),
    );

    const getLink = (post_id: string, comment_id?: string) => {
        let base = `https://reddit.com/r/${params.subreddit}/comments/${post_id}`;
        if (comment_id) {
            return base + "/-/" + comment_id;
        }
        return base;
    };

    const refreshStaffThread = (thread: StaffReplyThread) => {
        const makePromise = async () => {
            liveRefreshes++;
            try {
                return await refreshStaffReplyThread({
                    ...thread,
                    subreddit_id: sub?.id ?? "",
                    subreddit_name: params.subreddit,
                });
            } finally {
                liveRefreshes--;
            }
        };

        const innerPromise = makePromise();

        toast.promise(innerPromise, {
            loading: `Fetching any new or edited staff replies in ${thread.post_id}`,
            error: "Failed to fetch replies",
            success: (data) => {
                return `Saw ${data.total_comments} comments, ${data.total_staff_comments} by staff of which ${data.new_staff_comments} were new`;
            },
        });

        return innerPromise;
    };
</script>

<PagedTable query={threads} bind:page key={(i) => i.post_id}>
    {#snippet header()}
        <TableRow>
            <TableHead>Post</TableHead>
            <TableHead>Stickied Comment</TableHead>
            <TableHead>Commented At</TableHead>
            <TableHead>Suffix</TableHead>
            <TableHead></TableHead>
        </TableRow>
    {/snippet}

    {#snippet row(item: StaffReplyThread)}
        <TableRow>
            <TableCell>
                <Anchor href={getLink(item.post_id)}>{item.post_id}</Anchor>
            </TableCell>
            <TableHead>
                <Anchor href={getLink(item.post_id, item.our_comment_id)}>
                    {item.our_comment_id}
                </Anchor>
            </TableHead>
            <TableHead>{item.created_at}</TableHead>
            <TableHead>{item.suffix ? "yes" : "no"}</TableHead>
            <TableHead>
                <PromiseButton
                    disabled={liveRefreshes >= 3}
                    size="icon-sm"
                    onclick={async () => refreshStaffThread(item)}
                >
                    <RefreshCcw />
                </PromiseButton>
            </TableHead>
        </TableRow>
    {/snippet}
</PagedTable>
