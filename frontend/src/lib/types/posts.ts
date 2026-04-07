import * as z from "zod";

export const ZCreateSubredditPost = z.object({
  subreddit: z.string(),
  title: z.string(),
  content: z.string(),
  flair_id: z.string().optional(),
  flair_text: z.string().optional(),
  sticky: z.number(),
  distinguish: z.boolean(),
  lock: z.boolean(),
});

export const ZSubredditPost = ZCreateSubredditPost.extend({
  id: z.int32(),
  reddit_id: z.string().optional(),
  updated_at: z.iso.datetime(),
  synced_at: z.iso.datetime().optional(),
});

export const ZDbSubredditPost = ZSubredditPost.extend({
  flair_id: z.string().nullable(),
  flair_text: z.string().nullable(),
  reddit_id: z.string().nullable(),
  updated_at: z.date(),
  synced_at: z.date().nullable(),
});

export const ZSubredditPostStub = z.object({
  id: z.int32(),
  reddit_id: z.string().optional(),
  title: z.string(),
});

export type CreateSubredditPost = z.infer<typeof ZCreateSubredditPost>;
export type SubredditPost = z.infer<typeof ZSubredditPost>;
export type DbSubredditPost = z.infer<typeof ZDbSubredditPost>;
export type SubredditPostStub = z.infer<typeof ZSubredditPostStub>;
