import type { SidebarSubreddit } from "$lib/types/subreddit";
import { User } from "@lucide/svelte";
import { redirect, type ServerLoad } from "@sveltejs/kit";

export const load: ServerLoad = async ({ locals, parent }) => {
  if (!locals.user) {
    redirect(307, "/auth");
  }

  const { me, subs } = await parent();
  return {
    me: me as User | undefined,
    subs: (subs ?? []) as SidebarSubreddit[],
  };
};
