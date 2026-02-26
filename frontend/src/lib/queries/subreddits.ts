import type { ScamInfo, SubredditOptions } from "$lib/types/subreddit";

export async function fetchSubredditOptions(
  sub: string,
): Promise<SubredditOptions> {
  const response = await fetch(`/api/subreddits/${sub}`);

  if (!response.ok) {
    throw response;
  }

  return await response.json();
}

export async function fetchSubredditScams(sub: string): Promise<ScamInfo[]> {
  const response = await fetch(`/api/subreddits/${sub}/ocr`);

  if (!response.ok) {
    throw response;
  }

  return await response.json();
}
