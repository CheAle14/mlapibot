import { makeRedirectUrl } from "$lib/server/oauth";
import type { User } from "$lib/types/user";
import { redirect, type ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ locals }) => {
  const url = makeRedirectUrl();

  if (!locals.user) {
    redirect(307, url);
  }

  return {
    redirectUrl: url,
  };
};
