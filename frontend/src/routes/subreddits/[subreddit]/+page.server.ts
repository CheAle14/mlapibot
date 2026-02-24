import { getUserModSubreddits } from "$lib/server/database";
import type { SubredditOptions } from "$lib/types/subreddit";
import { redirect, type ServerLoad } from "@sveltejs/kit";

function makeTestData(): SubredditOptions {
  return {
    scams: {
      enabled: false,
      scams: [
        {
          id: 1,
          name: "Free Things",
          ocr: ["free nitro", "free boost"],
          report: true,
          remove: true,
        },
        {
          id: 2,
          name: "Account report",
          ocr: ["reported your account"],
          title: ["reported my account"],
          remove: true,
        },
        {
          id: 3,
          name: "Join server",
          title: ["join my server"],
          report: true,
        },
      ],
    },
    ai_slop: {
      enabled: true,
    },
    staff_reply: {
      enabled: false,
      flair_id: "",
      css_class: "",
    },
    status: {
      enabled: true,
      min_impact: "minor",
      distinguish: true,
      sticky: {
        min_impact: "major",
        delay_minor_mins: 15,
        comment_threshold: 999,
        delay_major_mins: 180,
      },
    },
    related_title: {
      enabled: false,
    },
  };
}

export const load: ServerLoad = async ({}) => {
  return {
    subdata: makeTestData(),
  };
};
