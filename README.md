### What is this
Source code for [/u/mlapibot](https://www.reddit.com/user/mlapibot) which uses OCR to determine text in images uploaded to /r/DiscordApp. Use a number of heurisitcs (lol jks, .Contains() 100%) to see if the text might be a scam

### How do I run this

Good question

### Data

At `data/scams.json`, we expect a file that contains the following JSON:


    {
        "scams": [
            {
                "name": "Some Name",
                "reason": "Why this is a scam, eg, 'poor grammar'",
                "text": ["some triggers", "that match into the words", "of the detected text",
                    "the trigger that matches the best will be used as the percentage match value"]
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
    
    
At `data/template.md`, we expect a markdown comment where {0} is replaced by a list of scams detected. An example is already present