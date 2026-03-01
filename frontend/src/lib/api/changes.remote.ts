import { command } from "$app/server";
import * as z from "zod";
import * as db from "$lib/server/database";
import { error } from "@sveltejs/kit";
import { isModeratorOf } from "./auth.remote";
import { ZPendingSubredditOptions } from "$lib/types/subreddit";
import { getSubredditOptions } from "./options.remote";
import { getSubredditScams } from "./scams.remote";
import { getTemplateStubs } from "./templates.remote";

import * as _ from "moderndash";

export const savePendingChanges = command(
  z.object({
    subreddit: z.string(),
    changes: ZPendingSubredditOptions,
  }),
  async ({ subreddit, changes }) => {
    if (!(await isModeratorOf(subreddit))) return error(403);

    const result = await db.tryApplyPendingChanges(subreddit, changes);

    if ("ok" in result) {
      getSubredditOptions(subreddit).refresh();

      if (!_.isEqual(changes.scams, {})) {
        getSubredditScams(subreddit).refresh();
      }

      if (!_.isEqual(changes.templates, {})) {
        getTemplateStubs(subreddit).refresh();
      }

      return result.ok;
    } else {
      return error(400, result.error);
    }
  },
);
