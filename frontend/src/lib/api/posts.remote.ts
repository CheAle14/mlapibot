import * as z from "zod";
import * as db from "$lib/server/database";
import { command, query } from "$app/server";
import {
  ZCreateSubredditPost,
  ZSubredditPost,
  type SubredditPost,
} from "$lib/types/posts";
import { error, invalid, redirect } from "@sveltejs/kit";
import { updateFirstInArrayBy } from "$lib/mutate";
import { isModeratorOf } from "./auth.remote";
import { API_URL } from "$env/static/private";

export const fetchSubPosts = query(z.string(), async (subreddit_id) => {
  if (!(await isModeratorOf(subreddit_id))) return error(403);

  return await db.getSubredditPosts(subreddit_id);
});

export const fetchSubPost = query(
  z.object({
    subreddit_id: z.string(),
    post_id: z.number(),
  }),
  async ({ subreddit_id, post_id }) => {
    if (!(await isModeratorOf(subreddit_id))) return error(403);

    return await db.getSubredditPost(subreddit_id, post_id);
  },
);

export const createSubPost = command(ZCreateSubredditPost, async (post) => {
  if (!(await isModeratorOf(post.subreddit))) return error(403);
  console.log("create", post);
  const result = await db.createSubredditPost(post);
  return { id: result.id };
});

export const updateSubPost = command(ZSubredditPost, async (post) => {
  if (!(await isModeratorOf(post.subreddit))) return error(403);

  console.log("update", post);
  return await db.updateSubredditPost(post);
});

export const publishSubPost = command(
  z.object({
    subreddit_id: z.string(),
    post_id: z.number(),
  }),
  async ({ subreddit_id, post_id }) => {
    if (!(await isModeratorOf(subreddit_id))) return error(403);
    const post = await db.getSubredditPost(subreddit_id, post_id);

    if (!post) {
      error(400, {
        message: "post does not exist",
      });
    }

    console.log("Publishing", post_id);
    const result = await fetch(API_URL + "/publish", {
      method: "POST",
      body: JSON.stringify({ id: post_id }),
    });

    if (result.ok) {
      const body = await result.json();
      return body as { id: string };
    } else {
      error(result.status, { message: "failed to publish" });
    }
  },
);

export const deleteSubPost = command(
  z.object({
    subreddit_id: z.string(),
    post_id: z.number(),
  }),
  async ({ subreddit_id, post_id }) => {
    if (!(await isModeratorOf(subreddit_id))) return error(403);
    await db.deleteSubPost(subreddit_id, post_id);
  },
);
