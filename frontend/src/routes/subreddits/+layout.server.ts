import { getUserModSubreddits } from "$lib/server/database";
import { redirect, type ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ locals }) => {
  if (!locals.user) {
    redirect(307, "/auth");
  }

  const subs = await getUserModSubreddits(locals.user.id);

  return {
    subs,
  };
};
