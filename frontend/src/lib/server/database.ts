import { DATABASE_URI } from "$env/static/private";
import {
  type DbSubreddit,
  type PendingSubredditOptions,
  type SubredditOptions,
  type UpdateScamInfo,
  type CreateScamInfo,
  type ScamInfo,
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

export async function getAllSubreddits() {
  const results = await sql<DbSubreddit[]>`
    SELECT *
    FROM subreddits sub
    `;

  return results;
}

function mapDataToOptions(sub: DbSubreddit): SubredditOptions {
  return {
    seq_num: sub.seq_num,
    scams: { ...sub.mod_scams, create: [], deletes: [], update: [] },
    ai_slop: sub.mod_ai_slop,
    status: sub.mod_status,
    staff_reply: sub.mod_staff_reply,
    related_title: sub.mod_related_title,
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

export async function getUserModSubreddits(user_id: string) {
  const results = await sql<DbSubreddit[]>`
    SELECT sub.*
    FROM subreddits sub
    JOIN subreddit_mods mods
    ON sub.id = mods.subreddit_id
    WHERE mods.user_id=${user_id}
    `;

  return results;
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

      if (deletes) {
        // TODO: figure out why dynamic 'where in' doesn't work.

        for (const id of deletes) {
          await sql`DELETE FROM subreddit_scam_rules WHERE id=${id}`;
        }
      }

      if (create) {
        const items = create.map((item) => ({
          ...item,
          subreddit_id: subreddit.id,
        }));
        await sql`INSERT INTO subreddit_scam_rules ${sql(items, "subreddit_id", "name", "ocr", "title", "remove", "report")}`;
      }

      if (update) {
        console.warn("TODO: update scams:", update);
      }

      subreddit.mod_scams = {
        ...subreddit.mod_scams,
        ...rest,
      };
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

    const next_seq = subreddit.seq_num + 1;

    await sql`
      UPDATE subreddits
      SET
        seq_num=${next_seq},
        mod_scams=${subreddit.mod_scams},
        mod_ai_slop=${subreddit.mod_ai_slop},
        mod_staff_reply=${subreddit.mod_staff_reply},
        mod_status=${subreddit.mod_status},
        mod_related_title=${subreddit.mod_related_title}
      WHERE id=${subreddit.id}
    `;

    subreddit.seq_num = next_seq;

    return { ok: mapDataToOptions(subreddit) };
  });
}
