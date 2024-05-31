const { https, setGlobalOptions } = require("firebase-functions/v2");
const { defineSecret } = require("firebase-functions/params");
const { initializeApp } = require("firebase-admin/app");
const { getFirestore } = require("firebase-admin/firestore");
const logger = require("firebase-functions/logger");

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
  const formData =
    `code=${code}&` +
    `client_id=${GITHUB_CLIENT_ID.value()}&` +
    `client_secret=${GITHUB_CLIENT_SECRET.value()}&` +
    "redirect_uri=http%3A//127.0.0.1";

  const result = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
      accept: "application/json",
    },
    body: formData,
  });

  return result.json();
}

const platforms = {
  github: githubExchange,
};

exports.getAuthToken = https.onRequest(async (req, res) => {
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
});
