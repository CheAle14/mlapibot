import { getTemplateStubs } from "$lib/components/templates/templates.remote";
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
    if (
      changes.templates.creates ||
      changes.templates.updates ||
      changes.templates.deletes
    ) {
      getTemplateStubs(subreddit).refresh();
    }

    return await response.json();
  } else {
    throw response;
  }
}

export async function createSubreddit(sub: { id: string; name: string }) {
  const response = await fetch(`/api/subreddits`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(sub),
  });

  if (response.ok) {
    return await response.json();
  } else {
    throw response;
  }
}

export async function addModerator(req: {
  subreddit_id: string;
  user_id: string;
}) {
  const response = await fetch(
    `/api/subreddits/${req.subreddit_id}/moderators/${req.user_id}`,
    {
      method: "PUT",
    },
  );

  if (!response.ok) {
    throw response;
  }
}
