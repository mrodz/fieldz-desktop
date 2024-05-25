const { onRequest } = require("firebase-functions/v2/https");

const { initializeApp } = require("firebase-admin/app");
const { getFirestore } = require("firebase-admin/firestore");

initializeApp();

exports.markScheduleRequest = onRequest(async (req, res) => {
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
