import * as db from "$lib/server/database";
import type { ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ cookies }) => {
  const cookie = cookies.get("mlapibot_auth");
  console.log("Auth cookie:", cookie);

  const me = cookie ? await db.getUserByCookie(cookie) : undefined;
  const subs = me ? await db.getUserModSubreddits(me.id) : undefined;

  console.log("me:", me);
  console.log("subs:", subs);

  return {
    me,
    subs,
  };
};
