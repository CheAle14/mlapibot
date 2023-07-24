### What is this
Source code for [/u/mlapibot](https://www.reddit.com/user/mlapibot) which uses OCR to determine text in images uploaded to /r/DiscordApp. 

Attempts to use a pseudo-[weighted levenshtein](https://github.com/luozhouyang/python-string-similarity#weighted-levenshtein) algorithm to compare the detected text against the known scam/trigger phrases.  
Two words are deemed 'similar' if the absolute distance between them is zero ("aa" == "aa"), one ("aa", "ba") or two ("dogs", "gods"), and the weighted distance is less than half.  
The algorithm first scans through the seen text and attempts to find the any of the first couple words in the trigger phrase.  
Then, for each starting pair (word in seen text, word in start of trigger), it steps through both the seen text and the trigger phase attempting to find each word in the trigger phrase in sequence in the seen text. If the two comparing words do not match, it will attempt to resolve by:

1. Considering whether the current seen word is a mere concatenation of the next two trigger words.
2. Considering whether the next trigger word can be found within the next couple seen words
3. Considering whether the current seen word can be skipped/ignored and we can find the same trigger word later on 

If none of these satisfactorily find the next trigger word, the algorithm stops this pair evaluation, summing up the distance value of the remainder of the trigger not found.   
During this pair evaluation the algorithm keeps track of the current actual distance as well as the maximum possible theoretical distance that could've been achieved had nothing matched. This allows it to calculate the 'percentage match' by `1 - (distance / maximum)`.   
To be considered as matching the trigger phrase, this percentage must be above 90%.  
The algorithm continues for all pairs, for all trigger phrases, finding the highest percentage.


### How do I run this

Install dependencies with `pip -r requirements.txt`  
You must also have installed and available on path [Tesseract-OCR](https://github.com/tesseract-ocr/tesseract), version 4.

Run `run.py`.  
With no arguments, this will use the praw configuration provided to monitor /r/DiscordApp or /r/mlapi for new posts to check.  
If a filepath or https:// URL to an image is given as an argument, it will analysed, displayed and then the program will exit.  

You can provide 'test' as the only argument to test all images in the tests folder.   
Any images in the `none` folder must trigger *zero* scams.  
Images in the other folders must trigger the scam whose `name` is equal to the folder's name.


### Data

At `mlapi/data/scams.json`, we expect a file that contains the following JSON:


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


At `mlapi/data/praw.ini`, we expect a file that contains the following:

    [bot1]
    client_id=...
    client_secret=...
    password=...
    username=...

You can optionally include a Discord webhook URL in `mlapi/data/webhook.txt` for messages to be sent there.  

At `mlapi/data/imgur.json` you can optionally put a JSON document containing:

    {
        "client_id": "<the imgur client id>",
        "client_secret": "<the imgur client secret>"
    }
    
    
We expect `*.md` files under the `data/templates` folder. Each scam has an optional 'template' json value, which defaults to 'default'. We respond to a post with the *last* scam detected's 'template'.md file.