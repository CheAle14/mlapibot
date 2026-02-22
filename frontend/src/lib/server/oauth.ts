import {
  CLIENT_ID,
  CLIENT_SECRET,
  OUR_BASE_URL,
  CLIENT_USERNAME,
  CLIENT_PASSWORD,
} from "$env/static/private";
import Cached from "$lib/cached";

const redirect_uri = OUR_BASE_URL + "/auth/callback";

interface TokenResponse {
  access_token: string;
}

interface ErrorResponse {
  error: string;
}

type AccessTokenResponse = TokenResponse;

export async function exchangeCodeForToken(code: string, state: string) {
  const response = await fetch("https://www.reddit.com/api/v1/access_token", {
    method: "POST",
    headers: {
      authorization: "Basic " + btoa(`${CLIENT_ID}:${CLIENT_SECRET}`),
      "content-type": "application/x-www-form-urlencoded",
    },
    body: `grant_type=authorization_code&code=${code}&redirect_uri=${encodeURIComponent(redirect_uri)}`,
  });

  const body = (await response.json()) as AccessTokenResponse;

  if (!response.ok) {
    throw response;
  }

  return body;
}

const OUR_TOKEN = new Cached(getClientCredentialsToken);

async function getClientCredentialsToken() {
  const response = await fetch("https://www.reddit.com/api/v1/access_token", {
    method: "POST",
    headers: {
      authorization: "Basic " + btoa(`${CLIENT_ID}:${CLIENT_SECRET}`),
      "content-type": "application/x-www-form-urlencoded",
    },
    body: `grant_type=password&username=${CLIENT_USERNAME}&password=${CLIENT_PASSWORD}`,
  });

  const body = (await response.json()) as AccessTokenResponse;

  console.log("clientCreds", body);

  if (!response.ok) {
    throw response;
  }

  return body;
}

interface RedditMe {
  id: string;
  name: string;
}

export async function fetchRedditMe(access_token: string) {
  const response = await fetch("https://oauth.reddit.com/api/v1/me.json", {
    headers: {
      authorization: `Bearer ${access_token}`,
    },
  });

  const body = (await response.json()) as RedditMe;

  if (!response.ok) {
    console.error("fetchRedditMe", body, response);
    throw response;
  }

  return body;
}

export async function fetchUserModeratedSubreddits(
  user: string,
  access_token: string,
) {
  const response = await fetch(
    `https://oauth.reddit.com/user/${user}/moderated_subreddits.json`,
    {
      headers: {
        authorization: `Bearer ${access_token}`,
      },
    },
  );

  const body = await response.json();

  if (!response.ok) {
    console.error("fetchUserModeratedSubreddits", user, body, response);
    throw response;
  }

  return body;
}

export function makeRedirectUrl() {
  return (
    `https://www.reddit.com/api/v1/authorize` +
    `?client_id=${CLIENT_ID}` +
    `&state=randomtexthere` +
    `&response_type=code` +
    `&redirect_uri=${encodeURIComponent(redirect_uri)}` +
    `&duration=temporary` +
    `&scope=identity`
  );
}
