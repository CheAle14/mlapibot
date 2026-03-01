import * as z from "zod";
import * as db from "$lib/server/database";
import { query } from "$app/server";
import { error } from "@sveltejs/kit";

export const getTemplateStubs = query(z.string(), async (subreddit) => {
  return await db.getSubredditTemplateStubs(subreddit);
});

export const getTemplateInfo = query(
  z.object({
    subreddit: z.string(),
    template: z.int32(),
  }),
  async ({ subreddit, template }) => {
    const result = await db.getSubredditTemplate(subreddit, template);

    if (result) {
      return result;
    } else {
      error(404);
    }
  },
);
