import { DATABASE_URI } from "$env/static/private";
import { type Subreddit } from "$lib/types/subreddit";
import { type DbUser, type User } from "$lib/types/user";
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
  const results = await sql<Subreddit[]>`
    SELECT sub.id, sub.name
    FROM subreddits sub
    `;

  return results;
}

export async function getUserModSubreddits(user_id: string) {
  const results = await sql<Subreddit[]>`
    SELECT sub.id, sub.name
    FROM subreddits sub
    JOIN subreddit_mods mods
    ON sub.id = mods.subreddit_id
    WHERE mods.user_id=${user_id}
    `;

  return results;
}
