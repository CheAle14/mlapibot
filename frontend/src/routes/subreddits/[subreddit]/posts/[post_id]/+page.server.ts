import { isModeratorOf } from "$lib/api/auth.remote";
import { error } from "@sveltejs/kit";
import type { PageServerLoad } from "./$types";
import * as db from "$lib/server/database";
import type { SubredditPost } from "$lib/types/posts";

export const load: PageServerLoad = async ({ params, parent }) => {
  const { subs } = await parent();

  const subreddit_id =
    subs.find((s) => s.name === params.subreddit)?.id ?? "<undefined>";

  const post_id = parseInt(params.post_id, 10);
  const post = await db.getSubredditPost(subreddit_id, post_id);

  if (!post) {
    error(404);
  }

  return { post: post as SubredditPost, subreddit_id };
};
