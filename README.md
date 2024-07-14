### What is this

Source code for [/u/mlapibot](https://www.reddit.com/user/mlapibot) which uses OCR to determine text in images uploaded to /r/DiscordApp.

Now re-written in Rust, it uses the Needleman–Wunsch alignment algorithm (modified to be word-based, and use fuzzy similarity via levenshtein) to compare detectected text against a list of known scam or other trigger phrases.

### How do I run this

This uses [leptess](https://crates.io/crates/leptess), which means you must have both leptonica and tesseract installed. This is a bit tricky on Windows unfortunately, good luck.  
After that, you should be able to use cargo to handle the other dependencies.

#### Analyzing on the CLI

You can either specify a file path to an image which will be OCRed for any scams through

    cargo run test --file <PATH>

or you can specify a http**S** link to an image which will be downloaded and then OCRed through

    cargo run test --link <SECURE LINK>

By default, the test command will write two files in the current direct:

- `seen.png`, which will be a copy of the input image with every word that the OCR detected drawn in a red bordered box.
- `trigger.png`, which will be another copy of the input image with every word that caused a scam to be detected bordered in red.

These paths can be changed with the `--seen` and `--trigger` commands respectively.

#### Reddit bot

Alternatively, you can run a reddit bot like /u/mlapibot through the reddit command:

    cargo run reddit --client_id <CID> --client_secret <CSRC> --username <USER> --password <PASS>

### Data

At `data/scams.json`, we expect a file that contains the following JSON:

    {
        "scams": [
            {
                "name": "Some Name",
                "reason": "Why this is a scam, eg, 'poor grammar'",
                "ocr": [
                    "some triggers",
                    "that match into the words",
                    "of the detected text",
                    "the trigger that matches the best will be used as the percentage match value",
                    "words with !exclamation !!marks are counted at !double !!triple !!!quadruple etc value
                    ],
                "report": true,
                "template": "default"
            },
            ...
        ]
    }

We expect `*.md` files under the `data/templates` folder. Each scam has an optional 'template' json value, which defaults to 'default'. The scam selected will be the one with the highest similarity score, or whichever reaches 100% first (since 100% cannot be improved on, the algorithm stops immediately).
