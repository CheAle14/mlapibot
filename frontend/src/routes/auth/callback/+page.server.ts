import { DEFAULT_ADMIN } from "$env/static/private";
import * as db from "$lib/server/database";
import {
  exchangeCodeForToken,
  fetchRedditMe,
  fetchUserModeratedSubreddits,
} from "$lib/server/oauth";
import { redirect, type ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ url, cookies }) => {
  const code = url.searchParams.get("code") ?? "";
  const state = url.searchParams.get("state") ?? "";

  const resp = await exchangeCodeForToken(code, state);
  const me = await fetchRedditMe(resp.access_token);

  let user = await db.upsertUser({
    id: me.id,
    name: me.name,
    admin: me.name === DEFAULT_ADMIN,
  });

  cookies.set("mlapibot_auth", user.cookie, {
    path: "/",
    secure: import.meta.env.PROD,
    maxAge: 3600 * 24 * 31,
  });

  // const subs = await fetchUserModeratedSubreddits(me.name, resp.access_token);

  redirect(307, "/");
};
