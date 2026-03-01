import type {
  ScamInfo,
  SubredditOptions,
  SubredditTemplate,
  SubredditTemplateStub,
} from "$lib/types/subreddit";

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

export async function fetchSubredditTemplateStubs(
  sub: string,
): Promise<SubredditTemplateStub[]> {
  const response = await fetch(`/api/subreddits/${sub}/templates`);

  if (!response.ok) {
    throw response;
  }

  return await response.json();
}

export async function fetchSubredditTemplate(
  sub: string,
  id: number,
): Promise<SubredditTemplate> {
  const response = await fetch(`/api/subreddits/${sub}/templates/${id}`);

  if (!response.ok) {
    throw response;
  }

  return await response.json();
}
