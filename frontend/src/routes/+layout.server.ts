import * as db from "$lib/server/database";
import type { ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ cookies }) => {
  const cookie = cookies.get("mlapibot_auth");

  const me = cookie ? await db.getUserByCookie(cookie) : undefined;
  const subs = me
    ? await db.getSidebarSubreddits(me.name, me.admin)
    : undefined;

  return {
    me,
    subs,
  };
};
