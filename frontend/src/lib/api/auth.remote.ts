import * as z from "zod";
import * as db from "$lib/server/database";
import { getRequestEvent, query } from "$app/server";

export const isAdmin = query(async () => {
  const { locals } = getRequestEvent();
  return locals.user?.admin ?? false;
});

export const isModeratorOf = query(z.string(), async (subreddit) => {
  const { locals } = getRequestEvent();

  if (!locals.user) {
    return false;
  }

  return (
    locals.user.admin ||
    (await db.isUserModeratorOf(subreddit, locals.user.name))
  );
});
