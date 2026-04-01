export interface GotRedditPost {
  id: string;
  subreddit_id: string;
  author: string;
  title: string;
  links?: string[];
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
  reason?: string;
}

export type PostAction = IgnoreAction | ActionData;

export interface OcrImageData {
  name: string;
  text: string;
  triggers: number[];
}

export interface GotAnalysis {
  action: PostAction;
  ocr?: OcrImageData[];
}
