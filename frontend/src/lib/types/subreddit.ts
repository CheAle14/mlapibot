import type { PartialDeep } from "type-fest";

export interface Subreddit {
  id: string;
  name: string;
}

export interface SubredditModule {
  enabled: boolean;
}

export interface ScamsModule extends SubredditModule {}

export interface StaffReplyModule extends SubredditModule {
  flair_id: string;
  css_class?: string;
}

export type StatusIncidentImpact =
  | "none"
  | "maintenance"
  | "minor"
  | "major"
  | "critical";

export interface StatusStickyConfig {
  replace_sticky?: string;
  comment_threshold: number;
  delay_minor_mins: number;
  delay_major_mins: number;
  min_impact?: StatusIncidentImpact;
  only_for?: string[];
}

export interface StatusModule extends SubredditModule {
  min_impact: StatusIncidentImpact;
  sticky?: StatusStickyConfig;
  distinguish: boolean;
}

export interface SubredditOptions {
  scams: ScamsModule;
  ai_slop: SubredditModule;
  staff_reply: StaffReplyModule;
  status: StatusModule;
  related_title: SubredditModule;
}

export type PendingSubredditOptions = {
  [P in keyof SubredditOptions]: Partial<SubredditOptions[P]>;
};
