import * as z from "zod";
import * as db from "$lib/server/database";
import { query } from "$app/server";
import { error } from "@sveltejs/kit";
import { isModeratorOf } from "./auth.remote";

export const getSubredditOptions = query(z.string(), async (subreddit) => {
  if (!(await isModeratorOf(subreddit))) return error(403);

  const sub = await db.getSubredditData(subreddit);

  if (!sub) {
    return error(404);
  }

  return sub;
});
