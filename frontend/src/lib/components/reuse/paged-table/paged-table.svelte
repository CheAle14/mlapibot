<script lang="ts" generics="T">
    import { Button } from "$lib/components/ui/button";
    import { Json } from "$lib/components/ui/json";
    import { Spinner } from "$lib/components/ui/spinner";
    import {
        Table,
        TableBody,
        TableCell,
        TableFooter,
        TableHead,
        TableHeader,
        TableRow,
    } from "$lib/components/ui/table";
    import type { PageReq, PageResp } from "$lib/types/pagination";
    import type { RemoteQuery } from "@sveltejs/kit";
    import type { Snippet } from "svelte";

    type Query = RemoteQuery<PageResp<T>>;

    interface Props {
        page: PageReq;
        query: Query;
        header: Snippet<[Query]>;
        row: Snippet<[T, number, PageResp<T>]>;

        skeletonRow?: Snippet<[number]>;

        alwaysShowPager?: boolean;

        key: (item: T) => string;
    }

    let {
        query,
        header,
        row,
        skeletonRow,
        key,
        page = $bindable(),
        alwaysShowPager = false,
    }: Props = $props();

    let previous = $state<PageResp<T> | undefined>(undefined);

    const nextPage = () => {
        previous = query.current;
        page.page++;
    };

    const prevPage = () => {
        previous = query.current;
        page.page--;
    };

    const renderQuery = $derived(query.current ?? previous);
    const maxPages = $derived(
        Math.floor((renderQuery?.total ?? 0) / page.limit - 1),
    );
</script>

<Table>
    <TableHeader>
        {@render header(query)}
    </TableHeader>

    <TableBody>
        {#if renderQuery}
            {#each renderQuery.data as item, index (key(item))}
                {@render row(item, index, renderQuery)}
            {/each}
        {:else if skeletonRow}
            {@render skeletonRow(0)}
            {@render skeletonRow(1)}
            {@render skeletonRow(2)}
        {/if}
    </TableBody>

    {#if alwaysShowPager || maxPages > 0}
        <TableFooter>
            <TableRow>
                <TableCell colspan={"100%" as any}>
                    <div class="flex justify-around gap-5">
                        <Button
                            onclick={prevPage}
                            disabled={query.loading || page.page === 0}
                        >
                            Prev</Button
                        >

                        <div>
                            {#if query.loading}
                                <Spinner />
                            {:else}
                                Page {page.page} of {maxPages}
                            {/if}
                        </div>

                        <Button
                            onclick={nextPage}
                            disabled={query.loading || page.page + 1 > maxPages}
                            >Next</Button
                        >
                    </div>
                </TableCell>
            </TableRow>
        </TableFooter>
    {/if}
</Table>
