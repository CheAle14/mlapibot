import {
  getSubredditScamRules,
  isUserModeratorOf,
} from "$lib/server/database.js";
import { error, json } from "@sveltejs/kit";
import { sleep } from "moderndash";

export async function GET({ locals, params }) {
  const { subreddit } = params;

  console.log(locals.user);
  if (
    !locals.user ||
    !(locals.user.admin || (await isUserModeratorOf(subreddit, locals.user.id)))
  ) {
    return error(400, { message: "bad subreddit" });
  }

  const rules = await getSubredditScamRules(subreddit);
  return json(rules);
}
