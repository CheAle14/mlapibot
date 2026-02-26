import type { PartialDeep } from "type-fest";

import * as z from "zod";

const DbScamInfo = z.object({
  id: z.int32(),
  name: z.string(),
  ocr: z.array(z.string()).nullable(),
  title: z.array(z.string()).nullable(),

  remove: z.boolean(),
  report: z.boolean(),
});

const BaseModule = z.object({
  enabled: z.boolean(),
});

const DbModuleScams = BaseModule.extend({});

const DbModuleAiSlop = BaseModule.extend({});

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

const DbModuleRelatedTitle = BaseModule.extend({});

const DbSubreddit = z.object({
  id: z.string(),
  name: z.string(),
  last_sync: z.iso.datetime(),

  seq_num: z.int32(),

  mod_json_schema: z.int32(),

  mod_scams: DbModuleScams,
  mod_ai_slop: DbModuleAiSlop,
  mod_staff_reply: DbModuleStaffReply,
  mod_status: DbModuleStatus,
  mod_related_title: DbModuleRelatedTitle,
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

const ApiSubreddit = DbSubreddit.omit({ mod_json_schema: true });

const ApiScamsModule = DbModuleScams.extend({
  create: z.array(CreateScamInfo),
  update: z.array(UpdateScamInfo),
  deletes: z.array(z.int32()),
});

const SubredditOptions = z.object({
  seq_num: z.int32(),
  scams: ApiScamsModule,
  ai_slop: DbModuleAiSlop,
  staff_reply: DbModuleStaffReply,
  status: DbModuleStatus,
  related_title: DbModuleRelatedTitle,
});

const PendingSubredditModules = z.object({
  scams: ApiScamsModule.partial(),
  ai_slop: DbModuleAiSlop.partial(),
  staff_reply: DbModuleStaffReply.partial(),
  status: DbModuleStatus.partial(),
  related_title: DbModuleRelatedTitle.partial(),
});

const ModuleKeyEnum = PendingSubredditModules.keyof();
type ModuleKeys = z.infer<typeof ModuleKeyEnum>;
export const ModuleKeys: ModuleKeys[] = [
  "scams",
  "ai_slop",
  "staff_reply",
  "status",
  "related_title",
] as const;

const PendingSubredditOptions = PendingSubredditModules.extend({
  seq_num: z.int32(),
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
