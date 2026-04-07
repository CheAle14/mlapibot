export interface IPhraseMatcher {
  type: "phrase";
  phrase: string;
}

export interface IAnyMatcher {
  type: "any";
  children: IMatcher[];
}

export interface IAllMatcher {
  type: "all";
  children: IMatcher[];
}

export interface IOrderedMatcher {
  type: "ordered";
  children: IMatcher[];
  max_steps?: number;
}

export type IMatcher =
  | IPhraseMatcher
  | IAllMatcher
  | IAnyMatcher
  | IOrderedMatcher;
