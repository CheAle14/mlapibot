import { API_URL } from "$env/static/private";
import * as z from "zod";
import * as db from "$lib/server/database";
import { command, query } from "$app/server";
import { error } from "@sveltejs/kit";
import { isModeratorOf } from "./auth.remote";
import type { GotRedditPost, PostAction } from "$lib/types/api";

export const getSubredditScams = query(z.string(), async (subreddit) => {
  if (!(await isModeratorOf(subreddit))) return error(403);
  return await db.getSubredditScamRules(subreddit);
});

export const fetchRedditSubmission = query(z.httpUrl(), async (link) => {
  console.log("Fetching", link);
  const result = await fetch(API_URL + "/get-reddit", {
    method: "POST",
    body: JSON.stringify({ link }),
  });

  const post = (await result.json()) as GotRedditPost;
  console.log("Got:", post);

  if (!(await isModeratorOf(post.subreddit_id))) return error(403);

  return post;
});

export const fetchScamAnalysisResult = command(
  z.object({
    subreddit_id: z.string(),
    title: z.string(),
    link: z.string().optional(),
    body: z.string().optional(),
  }),
  async (data) => {
    if (!(await isModeratorOf(data.subreddit_id))) return error(403);

    console.log("Sending for analysis", data);
    const result = await fetch(API_URL + "/analyze", {
      method: "POST",
      body: JSON.stringify(data),
    });

    if (!result.ok) {
      console.error(result);
      throw result;
    } else {
      return (await result.json()) as PostAction;
    }
  },
);
