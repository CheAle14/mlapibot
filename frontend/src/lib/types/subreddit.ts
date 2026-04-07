import * as z from "zod";
import { ZCreateTemplateInfo, ZEditTemplateInfo } from "./templates";

const ZRemovalReason = z.string();

export const ZScamInfo = z.object({
  id: z.int32(),
  name: z.string(),
  enabled: z.boolean(),
  self_post: z.boolean(),

  // Technically all of these are `IMatcher | null`, but
  // zod really did not like the recursive + discriminated union that is.
  // So for now we just accept anything and hope that it is correct.
  // Since it should only be modified via this frontend it shouldn't get out of
  // sync, so there shouldn't be any issues.
  ocr: z.json().optional(),
  title: z.json().optional(),
  body: z.json().optional(),
  title_or_body: z.json().optional(),

  reason: ZRemovalReason.optional(),
  template: z.int32().optional(),

  remove: z.boolean(),
  report: z.boolean(),
});

export type ScamInfo = z.infer<typeof ZScamInfo>;

export const ZSubredditTemplate = z.object({
  id: z.int32(),
  name: z.string(),
  content: z.string(),
});

export type DbSubredditTemplate = z.infer<typeof ZSubredditTemplate>;

export const ZSubredditTemplateStub = ZSubredditTemplate.omit({
  content: true,
});

export type SubredditTemplateStub = z.infer<typeof ZSubredditTemplateStub>;

const BaseModule = z.object({
  enabled: z.boolean(),
});

export const ZModuleScams = BaseModule.extend({
  search_modqueue: z.boolean().optional(),
});

export const ZModuleAiSlop = BaseModule.extend({
  modmail_to: z.string().optional(),
  report: z.boolean().default(false),
});

export const ZModuleRelatedTitle = BaseModule.extend({
  reason: ZRemovalReason,
  check_img_posts: z.boolean().default(false),
});

export const ZComplexCommentInfo = z.object({
  name: z.string(),
  link_title: z.array(z.string()),
  comment: z.array(z.string()),
  ignore_flairs: z.array(z.string()).optional(),
  reason: ZRemovalReason,
});

export type ComplexCommentInfo = z.infer<typeof ZComplexCommentInfo>;

export interface ComplexCommentModalItem extends ComplexCommentInfo {
  old_name?: string;
}

export const ZModuleComplexComments = BaseModule.extend({
  items: z.array(ZComplexCommentInfo).optional(),
});

export const ZModuleCommentsCdn = BaseModule.extend({});
export const ZModuleCommentsCode = BaseModule.extend({});

export const ZModuleStaffReply = BaseModule.extend({
  flair_id: z.string(),
  css_class: z.string().optional(),
  ignore_post_title_contains: z.array(z.string()),
});

export const ZStatusIncidentImpact = z.enum([
  "none",
  "maintenance",
  "minor",
  "major",
  "critical",
]);

export const ZStatusStickyConfig = z.object({
  replace_sticky: z.string().optional(),
  comment_threshold: z.int32().min(0),
  delay_minor_mins: z.int32().min(0),
  delay_major_mins: z.int32().min(0),
  min_impact: ZStatusIncidentImpact.optional(),
  only_for: z.array(z.string()).optional(),
});

export type StatusStickyConfig = z.infer<typeof ZStatusStickyConfig>;

export const ZModuleStatus = BaseModule.extend({
  min_impact: ZStatusIncidentImpact,
  sticky: ZStatusStickyConfig.optional(),
  distinguish: z.boolean(),
  flair_id: z.string().optional(),
});

export const ZSubreddit = z.object({
  id: z.string(),
  name: z.string(),
  last_sync: z.iso.datetime(),

  seq_num: z.int32(),

  mod_json_schema: z.int32(),

  removal_reasons: z.record(z.string(), z.string()),
  mod_scams: ZModuleScams,
  mod_ai_slop: ZModuleAiSlop,
  mod_staff_reply: ZModuleStaffReply,
  mod_status: ZModuleStatus,
  mod_related_title: ZModuleRelatedTitle,
  mod_complex_comments: ZModuleComplexComments,
  mod_comments_code: ZModuleCommentsCode,
  mod_comments_cdn: ZModuleCommentsCdn,
});
export type Subreddit = z.infer<typeof ZSubreddit>;

export const ZCreateScamInfo = ZScamInfo.extend({ id: z.uuidv4() });
export type CreateScamInfo = z.infer<typeof ZCreateScamInfo>;

export const ZUpdateScamInfo = ZScamInfo.partial({
  name: true,
  remove: true,
  report: true,
});
export type UpdateScamInfo = z.infer<typeof ZUpdateScamInfo>;

export const ZCreateOrUpdateScamInfo = z.discriminatedUnion("id", [
  ZCreateScamInfo,
  ZUpdateScamInfo,
]);
export type CreateOrUpdateScamInfo = z.infer<typeof ZCreateOrUpdateScamInfo>;

export interface SidebarSubreddit {
  id: string;
  name: string;
  is_mod: boolean;
}

export const ZApiScamsModule = ZModuleScams.extend({
  create: z.array(ZCreateScamInfo),
  update: z.array(ZUpdateScamInfo),
  deletes: z.array(z.int32()),
});

export const ZApiTemplateOptions = z.object({
  creates: z.array(ZCreateTemplateInfo),
  updates: z.array(ZEditTemplateInfo),
  deletes: z.array(z.int32()),
});

export const ZApiSubredditModules = z.object({
  scams: ZApiScamsModule,
  ai_slop: ZModuleAiSlop,
  staff_reply: ZModuleStaffReply,
  status: ZModuleStatus,
  related_title: ZModuleRelatedTitle,
  complex_comments: ZModuleComplexComments,
  comments_code: ZModuleCommentsCode,
  comments_cdn: ZModuleCommentsCdn,
});

export type ApiSubredditModules = z.infer<typeof ZApiSubredditModules>;

export const ZApiSubredditOptions = ZApiSubredditModules.extend({
  seq_num: z.int32(),
  removal_reasons: z.record(z.string(), z.string()),
  templates: ZApiTemplateOptions,
});

export type ApiSubredditOptions = z.infer<typeof ZApiSubredditOptions>;

const ModuleKeyEnum = ZApiSubredditModules.keyof();
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
