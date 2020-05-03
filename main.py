import praw
import logging
import re
import requests
import json
import tempfile
import os, sys, time
import ocr, scam

from datetime import datetime

os.chdir(os.path.join(os.getcwd(), "data"))

logging.basicConfig(filename='mlapi.log', level=logging.INFO)
logging.getLogger().addHandler(logging.StreamHandler(sys.stdout))
reddit = praw.Reddit("bot1", user_agent="script:mlapiOCR:v0.0.1 (by /u/DarkOverLordCO)")
subReddit = reddit.subreddit("discordapp")


valid_extensions = [".png", ".jpg", ".jpeg"]
SCAMS = []
THRESHOLD = 0.9

try:
    with open("scams.json") as f:
        rawText = f.read()
    obj = json.loads(rawText)
    for scm in obj["scams"]:
        SCAMS.append(scam.Scam(scm["name"], scm["reason"], scm["text"]))
except Exception as e:
    logging.error(e)
    SCAMS = []

if len(SCAMS) == 0:
    logging.error("Refusing to continue: no scams loaded")
    exit(1)

latest_done = []
handled_posts = []
handled_messages = []

try:
    with open("save.txt", "r") as f:
        for x in f:
            latest_done.append(x.rstrip())
            if len(latest_done) > 25:
                latest_done.pop(0)
except Exception as e:
    print(e)
    latest_done = []
    logging.warn("Failed to load previously handled things")

TEMPLATE = ""
try:
    with open("template.md", "r") as f:
        TEMPLATE = f.read()
except Exception as e:
    logging.error(e)

if not TEMPLATE:
    logging.error("Refusing to continue: Template is empty")
    exit(1)


def saveLatest(thingId):
    latest_done.append(thingId)
    if len(latest_done) > 25:
        latest_done.pop(0)
    with open("save.txt", "w") as f:
        f.write("\n".join(latest_done))

def loopInbox():
    inbox = reddit.inbox.unread()
    for post in inbox:
        print(post)

def validImage(url):
    for ext in valid_extensions:
        if url.endswith(ext):
            return True
    return False

def extractURLS(post):
    any_url = []
    if validImage(post.url):
        any_url.append(post.url)
    if post.is_self:
        matches = re.findall("https?:\/\/[\w\-%\.\/\=\?\&]+", post.selftext)
        for x in matches:
            if validImage(x):
                any_url.append(x)
    return any_url


def handleUrl(url):
    filename = url.split('/')[-1]
    r = requests.get(url, allow_redirects=True)
    if not r.ok:
        print("=== err")
        print(url)
        print(r)
        print("===")
        return
    tempPath = os.path.join(tempfile.gettempdir(), filename)
    print(tempPath)
    with open(tempPath, "wb") as f:
        f.write(r.content)
    text = ocr.getTextFromPath(tempPath, filename)
    array = [x.strip().lower() for x in text.split(' ')]
    print(array)
    scamResults = {}
    for x in SCAMS:
        result = x.PercentageMatch(array)
        if result > THRESHOLD:
            scamResults[x] = result
    return scamResults

def handlePost(post):
    print(post.title)
    urls = extractURLS(post)
    for url in urls:
        results = handleUrl(url)
        if len(results) > 0:
            text = ""
            for scam, confidence in results.items():
                text += scam.Name + ": " + scam.Reason + "\r\n\r\n"
                print(scam.Name, confidence)
            built = TEMPLATE.format(text)
            #post.reply(built)
            logging.info("Replied to: %s", post.title)
            return



def loopPosts():
    for post in subReddit.new(limit=25):
        if post.name in latest_done:
            break # Since we go new -> old, don't go any further into old
        logging.info("New: %s", post.title)
        saveLatest(post.name)
        handlePost(post)


if __name__ == "__main__":
    while True:
        logging.debug("Fetching in loop")
        loopPosts()
        loopInbox()



