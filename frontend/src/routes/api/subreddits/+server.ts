import type { CreateSubredditReq } from "$lib/types/settings.js";
import type { DbSubreddit } from "$lib/types/subreddit.js";
import * as db from "$lib/server/database";
import { error, json } from "@sveltejs/kit";

export async function POST({ locals, request }) {
  if (!locals.user || !locals.user.admin) {
    return error(400, { message: "unauthorised" });
  }

  const req = (await request.json()) as CreateSubredditReq;

  const sub: DbSubreddit = {
    ...req,
    last_sync: new Date().toISOString(),
    seq_num: 0,
    mod_json_schema: 0,
    removal_reasons: {},
    mod_scams: {
      enabled: false,
    },
    mod_ai_slop: {
      enabled: false,
    },
    mod_staff_reply: {
      enabled: false,
      flair_id: "",
    },
    mod_status: {
      enabled: false,
      min_impact: "none",
      distinguish: false,
    },
    mod_related_title: {
      enabled: false,
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

  return json(sub);
}
