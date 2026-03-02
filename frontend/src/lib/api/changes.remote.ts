import { command } from "$app/server";
import * as z from "zod";
import * as db from "$lib/server/database";
import { error } from "@sveltejs/kit";
import { isModeratorOf } from "./auth.remote";
import { ZApiSubredditOptions } from "$lib/types/subreddit";
import { getSubredditOptions } from "./options.remote";
import { getSubredditScams } from "./scams.remote";
import { getTemplateStubs } from "./templates.remote";

import * as _ from "moderndash";

type AnyArray = any[];

function anyEntries(...arrays: AnyArray[]): boolean {
  for (const array of arrays) {
    if (array.length > 0) return true;
  }

  return false;
}

export const savePendingChanges = command(
  z.object({
    subreddit: z.string(),
    changes: ZApiSubredditOptions,
  }),
  async ({ subreddit, changes }) => {
    if (!(await isModeratorOf(subreddit))) return error(403);

    const result = await db.tryApplyPendingChanges(subreddit, changes);

    if ("ok" in result) {
      getSubredditOptions(subreddit).refresh();

      if (
        anyEntries(
          changes.scams.create,
          changes.scams.update,
          changes.scams.deletes,
        )
      ) {
        getSubredditScams(subreddit).refresh();
      }

      if (
        anyEntries(
          changes.templates.creates,
          changes.templates.deletes,
          changes.templates.updates,
        )
      ) {
        getTemplateStubs(subreddit).refresh();
      }

      return result.ok;
    } else {
      return error(400, result.error);
    }
  },
);
