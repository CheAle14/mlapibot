import { command, query } from "$app/server";
import z from "zod";
import * as db from "$lib/server/database";
import { isModeratorOf } from "./auth.remote";
import { error } from "@sveltejs/kit";
import { sleep } from "moderndash";
import type { StaffReplyThread } from "$lib/types/staff_replies";
import { ZPageReq, type PageResp } from "$lib/types/pagination";
import { API_URL } from "$env/static/private";

export const fetchStaffReplyThreads = query(
  z.object({
    subreddit_name: z.string(),
    subreddit_id: z.string(),
    page: ZPageReq,
  }),
  async ({
    subreddit_name,
    subreddit_id,
    page,
  }): Promise<PageResp<StaffReplyThread>> => {
    if (!(await isModeratorOf(subreddit_id))) throw error(401);
    const threads = await db.getStaffReplyThreads(subreddit_name, page);
    return threads;
  },
);

export const refreshStaffReplyThread = command(
  z.object({
    subreddit_id: z.string(),
    subreddit_name: z.string(),
    post_id: z.string(),
  }),
  async (args) => {
    if (!(await isModeratorOf(args.subreddit_id))) throw error(401);

    const response = await fetch(API_URL + `/refresh-staff-reply`, {
      method: "POST",
      body: JSON.stringify(args),
    });

    console.log(response);

    if (!response.ok) throw error(response.status);

    return (await response.json()) as {
      total_comments: number;
      total_staff_comments: number;
      new_staff_comments: number;
    };
  },
);
