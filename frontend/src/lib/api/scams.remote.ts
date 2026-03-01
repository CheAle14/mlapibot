import * as z from "zod";
import * as db from "$lib/server/database";
import { query } from "$app/server";
import { error } from "@sveltejs/kit";
import { isModeratorOf } from "./auth.remote";

export const getSubredditScams = query(z.string(), async (subreddit) => {
  if (!(await isModeratorOf(subreddit))) return error(403);
  return await db.getSubredditScamRules(subreddit);
});
