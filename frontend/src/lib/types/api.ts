export interface GotRedditPost {
  id: string;
  subreddit_id: string;
  author: string;
  title: string;
  link?: string;
  body?: string;
}

export interface IgnoreAction {
  type: "ignore";
}

export interface ActionData {
  type: "action";
  analyser: string;
  module?: string;
  reply?: {
    text: string;
    distinguish: boolean;
  };
  moderate: "none" | "report" | "remove" | "filter";
}

export type PostAction = IgnoreAction | ActionData;
