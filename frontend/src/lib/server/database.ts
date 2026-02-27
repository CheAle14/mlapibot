import { DATABASE_URI } from "$env/static/private";
import {
  type DbSubreddit,
  type PendingSubredditOptions,
  type SubredditOptions,
  type UpdateScamInfo,
  type CreateScamInfo,
  type ScamInfo,
  type SidebarSubreddit,
} from "$lib/types/subreddit";
import { type DbUser, type User } from "$lib/types/user";
import { SubscriptIcon } from "@lucide/svelte";
import postgres from "postgres";

const sql = postgres(DATABASE_URI);

export async function upsertUser({ id, name, admin }: User): Promise<DbUser> {
  const [data]: [{ cookie: string; last_sync: Date }?] = await sql`
        INSERT INTO
            users (id, name, admin, cookie)
        VALUES
            ( ${id}, ${name}, ${admin}, gen_random_uuid() )
        ON CONFLICT (id) DO UPDATE
        SET
            name = ${name},
            cookie = gen_random_uuid()
        RETURNING cookie, last_sync;
    `;

  if (!data) throw "failed to create user";

  return {
    id,
    name,
    admin,
    ...data,
  };
}

export async function getUserByCookie(cookie: string) {
  const [result]: [User?] = await sql`
      SELECT id, name, admin
      FROM users
      WHERE cookie = ${cookie}
      `;

  return result;
}

export async function isUserModeratorOf(subreddit_id: string, user_id: string) {
  const result = await sql`
    SELECT COUNT(*) FROM subreddit_mods
    WHERE subreddit_id=${subreddit_id} AND user_id=${user_id}
  `;

  console.log(result);

  return result.length === 1 && result[0][0] === 1;
}

function mapDataToOptions(sub: DbSubreddit): SubredditOptions {
  return {
    seq_num: sub.seq_num,
    removal_reasons: sub.removal_reasons,
    scams: { ...sub.mod_scams, create: [], deletes: [], update: [] },
    ai_slop: sub.mod_ai_slop,
    status: sub.mod_status,
    staff_reply: sub.mod_staff_reply,
    related_title: sub.mod_related_title,
    comments_cdn: sub.mod_comments_cdn,
    comments_code: sub.mod_comments_code,
    complex_comments: sub.mod_complex_comments,
  };
}

export async function getSubredditData(
  subreddit: string,
): Promise<SubredditOptions | undefined> {
  const [sub]: [DbSubreddit?] = await sql`
    SELECT *
    FROM subreddits sub
    WHERE sub.id=${subreddit}
    `;

  if (sub) {
    return mapDataToOptions(sub);
  } else {
    return undefined;
  }
}

export async function getSidebarSubreddits(
  user_id: string,
  is_admin: boolean,
): Promise<SidebarSubreddit[]> {
  const join = is_admin ? sql`LEFT JOIN` : sql`JOIN`;

  const results = await sql<{ id: string; name: string; user_id?: string }[]>`
    SELECT sub.id, sub.name, mods.user_id
    FROM subreddits sub
    ${join} subreddit_mods mods
    ON sub.id = mods.subreddit_id
    WHERE mods.user_id=${user_id}
    `;

  return results.map((item) => ({ ...item, is_mod: !!item.user_id }));
}

export async function getSubredditScamRules(subreddit_id: string) {
  const results = await sql<ScamInfo[]>`
    SELECT *
    FROM subreddit_scam_rules
    WHERE subreddit_id=${subreddit_id}
    `;

  return results;
}

type AppliedResult = { ok: SubredditOptions } | { error: string };

export async function tryApplyPendingChanges(
  id: string,
  changes: PendingSubredditOptions,
): Promise<AppliedResult> {
  return sql.begin(async (sql) => {
    const [subreddit]: [DbSubreddit?] = await sql`
        SELECT *
        FROM subreddits
        WHERE id=${id}`;

    if (!subreddit) {
      return { error: "invalid subreddit" };
    }

    if (subreddit.seq_num !== changes.seq_num) {
      return {
        error: `mismatch sequence: has ${subreddit.seq_num} but you are updating ${changes.seq_num}`,
      };
    }

    if (changes.scams) {
      const { create, deletes, update, ...rest } = changes.scams;

      if (deletes && deletes.length > 0) {
        // TODO: figure out why dynamic 'where in' doesn't work.

        for (const id of deletes) {
          await sql`DELETE FROM subreddit_scam_rules WHERE id=${id}`;
        }
      }

      if (create && create.length > 0) {
        const items = create.map((item) => ({
          ...item,
          ocr: item.ocr ?? null,
          title: item.title ?? null,
          body: item.body ?? null,
          title_or_body: item.title_or_body ?? null,
          subreddit_id: subreddit.id,
        }));
        await sql`INSERT INTO subreddit_scam_rules ${sql(items, "subreddit_id", "name", "ocr", "title", "body", "title_or_body", "remove", "report")}`;
      }

      if (update) {
        console.warn("TODO: update scams:", update);
      }

      subreddit.mod_scams = {
        ...subreddit.mod_scams,
        ...rest,
      };
    }

    if (changes.removal_reasons) {
      const reasons = {
        ...subreddit.removal_reasons,
        ...(changes.removal_reasons.update ?? {}),
      };

      for (const rem of changes.removal_reasons.remove ?? []) {
        delete reasons[rem];
      }

      subreddit.removal_reasons = reasons;
    }

    if (changes.ai_slop) {
      subreddit.mod_ai_slop = {
        ...subreddit.mod_ai_slop,
        ...changes.ai_slop,
      };
    }

    if (changes.staff_reply) {
      subreddit.mod_staff_reply = {
        ...subreddit.mod_staff_reply,
        ...changes.staff_reply,
      };
    }

    if (changes.status) {
      subreddit.mod_status = {
        ...subreddit.mod_status,
        ...changes.status,
      };
    }

    if (changes.related_title) {
      subreddit.mod_related_title = {
        ...subreddit.mod_related_title,
        ...changes.related_title,
      };
    }

    if (changes.complex_comments) {
      subreddit.mod_complex_comments = {
        ...subreddit.mod_complex_comments,
        ...changes.complex_comments,
      };
    }

    if (changes.comments_code) {
      subreddit.mod_comments_code = {
        ...subreddit.mod_comments_code,
        ...changes.comments_code,
      };
    }

    if (changes.comments_cdn) {
      subreddit.mod_comments_cdn = {
        ...subreddit.mod_comments_cdn,
        ...changes.comments_cdn,
      };
    }

    subreddit.seq_num += 1;

    const { id: _id, ...update } = subreddit;

    await sql`
      UPDATE subreddits
      SET
        ${sql(update)}
      WHERE id=${subreddit.id}
    `;

    await sql.notify(
      "mlapibot",
      JSON.stringify({ type: "subreddit", id: subreddit.id }),
    );

    return { ok: mapDataToOptions(subreddit) };
  });
}
