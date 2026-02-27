import {
  getSubredditData,
  isUserModeratorOf,
  tryApplyPendingChanges,
} from "$lib/server/database.js";
import {
  ZPendingSubredditOptions,
  type SubredditOptions,
} from "$lib/types/subreddit";
import { error, json } from "@sveltejs/kit";

export async function GET({ locals, params }) {
  const { subreddit } = params;

  console.log(locals.user);
  if (
    !locals.user ||
    !(locals.user.admin || (await isUserModeratorOf(subreddit, locals.user.id)))
  ) {
    return error(400, { message: "bad subreddit" });
  }

  const sub = await getSubredditData(subreddit);

  if (sub) {
    return json(sub);
  } else {
    return error(404);
  }
}

export async function PATCH({ locals, params, request }) {
  const { subreddit } = params;

  if (
    !locals.user ||
    !(locals.user.admin || (await isUserModeratorOf(subreddit, locals.user.id)))
  ) {
    return error(400, { message: "bad subreddit" });
  }

  const input = await request.json();

  const parsed = ZPendingSubredditOptions.safeParse(input);

  if (parsed.error) {
    return error(400, parsed.error);
  }

  const { data } = parsed;

  const result = await tryApplyPendingChanges(subreddit, data);

  if (result.ok) {
    return json(result.ok);
  } else {
    return error(400, result.error);
  }
}
