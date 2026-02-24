import { getAllSubreddits, getUserModSubreddits } from "$lib/server/database";
import { redirect, type ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ locals }) => {
  if (!locals.user) {
    redirect(307, "/auth");
  }

  const subs = locals.user.admin
    ? await getAllSubreddits()
    : await getUserModSubreddits(locals.user.id);

  return {
    subs,
  };
};
