import type { PartialDeep } from "type-fest";
import type { Deletable } from "./deletable";

export interface Subreddit {
  id: string;
  name: string;
}

export interface SubredditModule {
  enabled: boolean;
}

export interface ScamInfo extends Deletable {
  id: number;
  name: string;
  ocr?: string[];
  title?: string[];

  remove?: boolean;
  report?: boolean;
}

export interface ScamsModule extends SubredditModule {
  scams: ScamInfo[];
}

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

export type PartialExceptKey<T, K extends keyof T> = Partial<T> & Pick<T, K>;

export type PendingSubredditOptions = {
  scams: Partial<Omit<ScamsModule, "scams">> & {
    scams?: PartialExceptKey<ScamInfo, "id">[];
  };
  ai_slop: Partial<SubredditModule>;
  staff_reply: Partial<StaffReplyModule>;
  status: Partial<StatusModule>;
  related_title: Partial<SubredditModule>;
};
