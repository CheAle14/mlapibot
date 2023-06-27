### What is this
Source code for [/u/mlapibot](https://www.reddit.com/user/mlapibot) which uses OCR to determine text in images uploaded to /r/DiscordApp. 

Attempts to use the [weighted levenshtein](https://github.com/luozhouyang/python-string-similarity#weighted-levenshtein) algorithm to compare the detected text against the known scam/trigger phrases.

### How do I run this

Run `run.py`.  
You can pass a filepath or URL to an image as an argument and it will just analyse that image, then exit.

### Data

At `data/scams.json`, we expect a file that contains the following JSON:


    {
        "scams": [
            {
                "name": "Some Name",
                "reason": "Why this is a scam, eg, 'poor grammar'",
                "text": ["some triggers", "that match into the words", "of the detected text",
                    "the trigger that matches the best will be used as the percentage match value"],
                "template": "default" 
            },
            ...
        ]
    }


At `data/praw.ini`, we expect a file that contains the following:

    [bot1]
    client_id=...
    client_secret=...
    password=...
    username=...
    
    
We expect `*.md` files under the `data/templates` folder. Each scam has an optional 'template' json value, which defaults to 'default'. We respond to a post with the *last* scam detected's 'template'.md file.