import type { PendingSubredditOptions } from "$lib/types/subreddit";

interface SyncPendingChangesArgs {
  subreddit: string;
  changes: PendingSubredditOptions;
}

export async function syncPendingChanges({
  subreddit,
  changes,
}: SyncPendingChangesArgs) {
  const response = await fetch(`/api/subreddits/${subreddit}`, {
    method: "PATCH",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(changes),
  });

  if (response.ok) {
    return await response.json();
  } else {
    throw response;
  }
}
