import { makeRedirectUrl } from "$lib/server/oauth";
import type { User } from "$lib/types/user";
import type { ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ locals }) => {
  return {
    redirectUrl: makeRedirectUrl(),
  };
};
