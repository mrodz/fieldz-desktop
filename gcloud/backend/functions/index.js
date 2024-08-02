const { https, setGlobalOptions } = require("firebase-functions/v2");
const { defineSecret } = require("firebase-functions/params");
const { initializeApp } = require("firebase-admin/app");
const { getFirestore } = require("firebase-admin/firestore");
const logger = require("firebase-functions/logger");
const OAuth = require("oauth-1.0a");
const crypto = require("crypto");

setGlobalOptions({
  cpu: 1,
  memory: "128MiB",
  maxInstances: 1,
  timeoutSeconds: 5,
});

initializeApp();

exports.markScheduleRequest = https.onRequest(async (req, res) => {
  if (!("uid" in req.query)) {
    res.status(400).send("Missing uid query parameter");
    return;
  }

  const uid = req.query.uid;

  const userRef = getFirestore().collection("usage").doc(String(uid));

  try {
    const thisUser = await userRef.get();

    if (!thisUser.exists) {
      const newEntry = {
        runs: 1,
      };

      userRef.create(newEntry);

      res.json(newEntry);
    } else {
      const existingEntry = thisUser.data();

      existingEntry.runs = (existingEntry?.runs ?? 0) + 1;

      await userRef.update(existingEntry);

      res.json(existingEntry);
    }
  } catch (e) {
    res.status(500).json(e);
  }
});

const GITHUB_CLIENT_ID = defineSecret("GITHUB_CLIENT_ID");
const GITHUB_CLIENT_SECRET = defineSecret("GITHUB_CLIENT_SECRET");

async function githubExchange(code) {
  const clientId = GITHUB_CLIENT_ID.value();
  const clientSecret = GITHUB_CLIENT_SECRET.value();

  const result = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({
      code,
      client_id: clientId,
      client_secret: clientSecret,
    }),
  });

  return result.json();
}

const platforms = {
  github: githubExchange,
};

exports.getAuthToken = https.onRequest(
  { secrets: ["GITHUB_CLIENT_ID", "GITHUB_CLIENT_SECRET"] },
  async (req, res) => {
    if (!("platform" in req.query)) {
      res.status(400).send("Missing `platform` query parameter");
      return;
    }

    if (!("code" in req.query)) {
      res.status(400).send("Missing `code` query parameter");
      return;
    }

    if (!(req.query.platform in platforms)) {
      res.status(400).send(`Unknown platform: ${req.query.platform}`);
      return;
    }

    const resolver = platforms[req.query.platform];

    try {
      const transfer = await resolver(req.query.code);

      logger.info(
        `Succesfully completed auth transfer: ${JSON.stringify(transfer)}`
      );

      if (typeof transfer === "object" && "error" in transfer) {
        res.status(500).json(transfer);
        return;
      }

      res.status(200).json({
        access_token: transfer.access_token,
      });
    } catch (error) {
      logger.error("Internal error fetching access_token", error);
      res.status(500).json({
        error,
      });
    }
  }
);

const TWITTER_CONSUMER_KEY = defineSecret("TWITTER_CONSUMER_KEY");
const TWITTER_CONSUMER_SECRET = defineSecret("TWITTER_CONSUMER_SECRET");

exports.getTwitterRequestToken = https.onRequest(
  {
    secrets: ["TWITTER_CONSUMER_KEY", "TWITTER_CONSUMER_SECRET"],
  },
  async (req, res) => {
    if (!("port" in req.query && Number.isInteger(Number(req.query.port)))) {
      return res.status(400).json({
        error: "Missing parameter: port(int)",
      });
    }

    const request_data = {
      url: "https://api.twitter.com/oauth/request_token",
      method: "POST",
      data: {
        oauth_callback: `http://127.0.0.1:${req.query.port}`,
      },
    };

    const oauth = OAuth({
      consumer: {
        key: TWITTER_CONSUMER_KEY.value(),
        secret: TWITTER_CONSUMER_SECRET.value(),
      },
      signature_method: "HMAC-SHA1",
      hash_function(base_string, key) {
        return crypto
          .createHmac("sha1", key)
          .update(base_string)
          .digest("base64");
      },
    });

    const response = await fetch(request_data.url, {
      method: request_data.method,
      headers: oauth.toHeader(oauth.authorize(request_data)),
    });

    if (!response.ok) {
      return res.status(response.status).json({
        error: "The backend could not communicate with the twitter API",
      });
    }

    const text = await response.text();

    const responseParams = new URLSearchParams(text);
    const requestToken = responseParams.get("oauth_token");
    const requestTokenSecret = responseParams.get("oauth_token_secret");

    res.json({
      data: {
        oauth_token: requestToken,
        oauth_token_secret: requestTokenSecret,
        authorization_url: `https://api.twitter.com/oauth/authorize?oauth_token=${requestToken}`,
      },
    });
  }
);

exports.getTwitterOAuthCredentials = https.onRequest(
  {
    secrets: ["TWITTER_CONSUMER_KEY", "TWITTER_CONSUMER_SECRET"],
  },
  async (req, res) => {
    if (
      !(
        "oauth_token" in req.query &&
        "oauth_token_secret" in req.query &&
        "oauth_verifier" in req.query
      )
    ) {
      return res.status(400).json({
        error:
          "Missing parameters: oauth_token(str), oauth_token_secret(str), oauth_verifier(str)",
      });
    }

    const { oauth_token, oauth_token_secret, oauth_verifier } = req.query;

    const access_token_request_data = {
      url: "https://api.twitter.com/oauth/access_token",
      method: "POST",
      data: {
        oauth_verifier,
      },
    };

    const oauth = OAuth({
      consumer: {
        key: TWITTER_CONSUMER_KEY.value(),
        secret: TWITTER_CONSUMER_SECRET.value(),
      },
      signature_method: "HMAC-SHA1",
      hash_function(base_string, key) {
        return crypto
          .createHmac("sha1", key)
          .update(base_string)
          .digest("base64");
      },
    });

    const authorizedData = oauth.authorize(access_token_request_data, {
      key: oauth_token,
      secret: oauth_token_secret,
    }); 

    const response = await fetch(access_token_request_data.url, {
      method: access_token_request_data.method,
      headers: oauth.toHeader(authorizedData),
    });

    if (!response.ok) {
      return res.status(response.status).json({
        error: "The backend could not communicate with the twitter API",
      });
    }

    const text = await response.text();

    const responseParams = new URLSearchParams(text);
    const oauthToken = responseParams.get("oauth_token");
    const oauthTokenSecret = responseParams.get("oauth_token_secret");

    res.json({
      data: {
        oauth_token: oauthToken,
        oauth_token_secret: oauthTokenSecret,
      },
    });
  }
);
