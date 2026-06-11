import z from "zod";

export const ZPageReq = z.object({
  page: z.number().nonnegative(),
  limit: z.number().nonnegative().lt(100),
});

export type PageReq = z.infer<typeof ZPageReq>;

export interface PageResp<T> {
  total: number;
  data: T[];
}
