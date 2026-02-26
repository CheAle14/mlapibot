import * as db from "$lib/server/database";
import type { Handle } from "@sveltejs/kit";

export const handle: Handle = async ({ event, resolve }) => {
  const cookie = event.cookies.get("mlapibot_auth");

  if (cookie) {
    event.locals.user = await db.getUserByCookie(cookie);
  }

  console.log(event.request.method, event.url.pathname, event.locals.user);

  const response = await resolve(event);
  return response;
};
