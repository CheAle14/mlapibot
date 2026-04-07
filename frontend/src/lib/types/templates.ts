import * as z from "zod";

export const ZTemplateInfo = z.object({
  id: z.int32(),
  name: z.string(),
  content: z.string(),
});

export type TemplateInfo = z.infer<typeof ZTemplateInfo>;

export const ZCreateTemplateInfo = z.object({
  id: z.string(),
  name: z.string(),
  content: z.string(),
});

export type CreateTemplateInfo = z.infer<typeof ZCreateTemplateInfo>;

export const ZEditTemplateInfo = z.object({
  id: z.int32(),
  name: z.string(),
  content: z.string(),
});

export type EditTemplateInfo = z.infer<typeof ZEditTemplateInfo>;

export interface CreateOrStubTemplateInfo {
  id: string | number;
  name: string;
}

export function isCreatingTemplate(
  v: CreateOrStubTemplateInfo,
): v is CreateTemplateInfo {
  return typeof v.id === "string";
}
