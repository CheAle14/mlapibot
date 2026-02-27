import type { PartialDeep } from "type-fest";

import * as z from "zod";

const DbScamInfo = z.object({
  id: z.int32(),
  name: z.string(),

  // Technically all of these are `IMatcher | null`, but
  // zod really did not like the recursive + discriminated union that is.
  // So for now we just accept anything and hope that it is correct.
  // Since it should only be modified via this frontend it shouldn't get out of
  // sync, so there shouldn't be any issues.
  ocr: z.json().optional(),
  title: z.json().optional(),
  body: z.json().optional(),
  title_or_body: z.json().optional(),

  template: z.string().optional(),

  remove: z.boolean(),
  report: z.boolean(),
});

const BaseModule = z.object({
  enabled: z.boolean(),
});

const DbModuleScams = BaseModule.extend({});
const DbModuleAiSlop = BaseModule.extend({});
const DbModuleRelatedTitle = BaseModule.extend({});
const DbModuleComplexComments = BaseModule.extend({});
const DbModuleCommentsCdn = BaseModule.extend({});
const DbModuleCommentsCode = BaseModule.extend({});

const DbModuleStaffReply = BaseModule.extend({
  flair_id: z.string(),
  css_class: z.string().optional(),
});

const StatusIncidentImpact = z.enum([
  "none",
  "maintenance",
  "minor",
  "major",
  "critical",
]);

const StatusStickyConfig = z.object({
  replace_sticky: z.string().optional(),
  comment_threshold: z.int32().min(0),
  delay_minor_mins: z.int32().min(0),
  delay_major_mins: z.int32().min(0),
  min_impact: StatusIncidentImpact.optional(),
  only_for: z.array(z.string()).optional(),
});

const DbModuleStatus = BaseModule.extend({
  min_impact: StatusIncidentImpact,
  sticky: StatusStickyConfig.optional(),
  distinguish: z.boolean(),
});

const DbSubreddit = z.object({
  id: z.string(),
  name: z.string(),
  last_sync: z.iso.datetime(),

  seq_num: z.int32(),

  mod_json_schema: z.int32(),

  removal_reasons: z.record(z.string(), z.string()),
  mod_scams: DbModuleScams,
  mod_ai_slop: DbModuleAiSlop,
  mod_staff_reply: DbModuleStaffReply,
  mod_status: DbModuleStatus,
  mod_related_title: DbModuleRelatedTitle,
  mod_complex_comments: DbModuleComplexComments,
  mod_comments_code: DbModuleCommentsCode,
  mod_comments_cdn: DbModuleCommentsCdn,
});

const ApiScamInfo = DbScamInfo;

const CreateScamInfo = ApiScamInfo.extend({ id: z.uuidv4() });

const UpdateScamInfo = ApiScamInfo.partial({
  name: true,
  remove: true,
  report: true,
});

const CreateOrUpdateScamInfo = z.discriminatedUnion("id", [
  CreateScamInfo,
  UpdateScamInfo,
]);

export interface SidebarSubreddit {
  id: string;
  name: string;
  is_mod: boolean;
}

const ApiSubreddit = DbSubreddit.omit({ mod_json_schema: true });

const ApiScamsModule = DbModuleScams.extend({
  create: z.array(CreateScamInfo),
  update: z.array(UpdateScamInfo),
  deletes: z.array(z.int32()),
});

const SubredditOptions = z.object({
  seq_num: z.int32(),
  removal_reasons: z.record(z.string(), z.string()),
  scams: ApiScamsModule,
  ai_slop: DbModuleAiSlop,
  staff_reply: DbModuleStaffReply,
  status: DbModuleStatus,
  related_title: DbModuleRelatedTitle,
  complex_comments: DbModuleComplexComments,
  comments_code: DbModuleCommentsCode,
  comments_cdn: DbModuleCommentsCdn,
});

const PendingSubredditModules = z.object({
  scams: ApiScamsModule.partial(),
  ai_slop: DbModuleAiSlop.partial(),
  staff_reply: DbModuleStaffReply.partial(),
  status: DbModuleStatus.partial(),
  related_title: DbModuleRelatedTitle.partial(),
  complex_comments: DbModuleComplexComments.partial(),
  comments_code: DbModuleCommentsCode.partial(),
  comments_cdn: DbModuleCommentsCdn.partial(),
});

const ModuleKeyEnum = PendingSubredditModules.keyof();
type ModuleKeys = z.infer<typeof ModuleKeyEnum>;
export const ModuleKeys: ModuleKeys[] = [
  "scams",
  "ai_slop",
  "staff_reply",
  "status",
  "related_title",
  "complex_comments",
  "comments_code",
  "comments_cdn",
] as const;

const PendingSubredditOptions = PendingSubredditModules.extend({
  seq_num: z.int32(),
  removal_reasons: z
    .object({
      update: z.record(z.string(), z.string()),
      remove: z.array(z.string()),
    })
    .partial(),
});

type TDbSubreddit = z.infer<typeof DbSubreddit>;

type TScamInfo = z.infer<typeof ApiScamInfo>;

type TCreateScamInfo = z.infer<typeof CreateScamInfo>;
type TUpdateScamInfo = z.infer<typeof UpdateScamInfo>;
type TCreateOrUpdateScamInfo = z.infer<typeof CreateOrUpdateScamInfo>;
type TSubredditOptions = z.infer<typeof SubredditOptions>;
type TPendingSubredditModules = z.infer<typeof PendingSubredditModules>;
type TPendingSubredditOptions = z.infer<typeof PendingSubredditOptions>;

export type {
  TDbSubreddit as DbSubreddit,
  TSubredditOptions as SubredditOptions,
  TScamInfo as ScamInfo,
  TPendingSubredditModules as PendingSubredditModules,
  TPendingSubredditOptions as PendingSubredditOptions,
  TCreateOrUpdateScamInfo as CreateOrUpdateScamInfo,
  TCreateScamInfo as CreateScamInfo,
  TUpdateScamInfo as UpdateScamInfo,
};

export { PendingSubredditOptions as ZPendingSubredditOptions };
