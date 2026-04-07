import { form, query } from "$app/server";
import * as z from "zod";
import * as db from "$lib/server/database";
import { error } from "@sveltejs/kit";
import { isAdmin, isModeratorOf } from "./auth.remote";
import type { Subreddit } from "$lib/types/subreddit";

export const createSubreddit = form(
  z.object({
    id: z.string(),
    name: z.string(),
  }),
  async ({ id, name }) => {
    if (!(await isAdmin())) return error(403);

    const sub: Subreddit = {
      id,
      name,
      last_sync: new Date().toISOString(),
      seq_num: 0,
      mod_json_schema: 0,
      removal_reasons: {},
      mod_scams: {
        enabled: false,
      },
      mod_ai_slop: {
        enabled: false,
        report: false,
      },
      mod_staff_reply: {
        enabled: false,
        flair_id: "",
        ignore_post_title_contains: [],
      },
      mod_status: {
        enabled: false,
        min_impact: "none",
        distinguish: false,
      },
      mod_related_title: {
        enabled: false,
        reason: "",
        check_img_posts: false,
      },
      mod_complex_comments: {
        enabled: false,
      },
      mod_comments_code: {
        enabled: false,
      },
      mod_comments_cdn: {
        enabled: false,
      },
    };

    await db.createSubreddit(sub);

    return sub;
  },
);

export const addSubredditModerator = form(
  z.object({
    subreddit_id: z.string(),
    username: z.string(),
  }),
  async ({ subreddit_id, username }) => {
    if (!(await isAdmin())) return error(403);

    await db.addModerator(subreddit_id, username);
  },
);
