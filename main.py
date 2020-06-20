import praw
import logging
import re
import requests
import json
import tempfile
import os, sys, time
import ocr

from datetime import datetime
from praw.models import Message, Comment
from webhook import WebhookSender
from requests.adapters import HTTPAdapter
from requests.packages.urllib3.util.retry import Retry
from models import Scam, ScamEncoder

os.chdir(os.path.join(os.getcwd(), "data"))

valid_extensions = [".png", ".jpg", ".jpeg"]
def load_reddit():
    global reddit, subReddit, author
    author = "DarkOverLordCO"
    reddit = praw.Reddit("bot1", user_agent="script:mlapiOCR:v0.0.3 (by /u/" + author + ")")
    subReddit = reddit.subreddit("DiscordApp")
def load_scams():
    global SCAMS, THRESHOLD
    SCAMS = []
    THRESHOLD = 0.9

    try:
        with open("scams.json") as f:
            rawText = f.read()
        obj = json.loads(rawText)
        for scm in obj["scams"]:
            template = scm.get("template", "default")
            if "name" in scm:
                SCAMS.append(Scam(scm["name"], scm["reason"], scm["text"], template))
            else:
                SCAMS.append(Scam(scm["Name"], scm["Reason"], scm["Texts"], template))
    except Exception as e:
        logging.error(e)
        print(e)
        SCAMS = []

    if len(SCAMS) == 0:
        logging.error("Refusing to continue: no scams loaded")
        exit(1)

def save_scams():
    try:
        content = json.dumps({"scams": SCAMS}, indent=4, cls=ScamEncoder)
        with open("scams.json", "w") as f:
            f.write(content)
    except Exception as e:
        logging.error(e)

def load_history():
    global HISTORY, HISTORY_TOTAL, TOTAL_CHECKS
    HISTORY = {}
    HISTORY_TOTAL = 0
    TOTAL_CHECKS = 0
    try:
        with open("history.json", "r") as f:
            content = f.read()
    except:
        return
    obj = json.loads(content)
    HISTORY = obj["history"]
    HISTORY_TOTAL = obj["scams"]
    TOTAL_CHECKS = obj["total"]

def save_history():
    content = json.dumps({
        "history": HISTORY,
        "total": TOTAL_CHECKS,
        "scams": HISTORY_TOTAL
    })
    try:
        with open ("history.json", "w") as f:
            f.write(content)
    except Exception as e:
        logging.error(e)
        return
def load_templates():
    global TEMPLATES
    TEMPLATES = {}
    files = os.listdir("templates")
    for x in files:
        if x.endswith(".md"):
            name = x[:-3]
            print(name)
            with open("templates/" + x, "r") as f:
                TEMPLATES[name] = f.read()
    return TEMPLATES

def setup():
    global webHook, WEBHOOK_URL, latest_done, handled_messages, handled_posts,\
        TEMPLATES

    load_scams()
    load_reddit()

    try:
        with open("webhook.txt", "r") as f:
            WEBHOOK_URL = f.read()
    except Exception as e:
        logging.error(e)
        logging.warning("Disabling webhook sending as missing URL")
        WEBHOOK_URL = None

    webHook = WebhookSender(WEBHOOK_URL, subReddit.display_name)

    latest_done = []
    handled_posts = []
    handled_messages = []

    try:
        with open("save.txt", "r") as f:
            for x in f:
                latest_done.append(x.rstrip())
                if len(latest_done) > 50:
                    latest_done.pop(0)
    except Exception as e:
        logging.error(e)
        latest_done = []
        logging.warn("Failed to load previously handled things")

    try:
        TEMPLATES = load_templates()
    except Exception as e:
        logging.error(e)

    if not TEMPLATES:
        logging.error("Refusing to continue: Templates is empty")
        exit(1)

    load_history()

def requests_retry_session(
    retries=3,
    backoff_factor=2,
    status_forcelist=(500, 502, 504),
    session=None,
    ):
    session = session or requests.Session()
    retry = Retry(
        total=retries,
        read=retries,
        connect=retries,
        backoff_factor=backoff_factor,
        status_forcelist=status_forcelist,
    )
    adapter = HTTPAdapter(max_retries=retry)
    session.mount('http://', adapter)
    session.mount('https://', adapter)
    return session


def saveLatest(thingId):
    latest_done.append(thingId)
    if len(latest_done) > 50:
        latest_done.pop(0)
    with open("save.txt", "w") as f:
        f.write("\n".join(latest_done))

def addScam(content):
    lines = content.split("\n")
    name = lines[1]
    reason = lines[2]
    texts = lines[3:]
    scm = Scam(name, reason, texts, None)
    SCAMS.append(scm)
    save_scams()

def handleAdminMsg(post):
    if post.body.startswith("[add]"):
        addScam(post.body)
        post.reply("Registered new scam; note: will not persist.")


def loopInbox():
    unread_messages = []
    for item in reddit.inbox.unread(limit=None):
        if isinstance(item, Message):
            unread_messages.append(item)
        if isinstance(item, Comment):
            unread_messages.append(item)
    reddit.inbox.mark_read(unread_messages)
    for x in unread_messages:
        webHook.sendInboxMessage(x)
        body = str(str(x.body).encode("utf-8"))
        logging.warning("%s: %s", x.author.name, body)
        if x.author.name == author:
            handleAdminMsg(x)

def getFileName(url):
        index = url.rfind('/')
        if index == -1:
            index = url.rfind('\\')
        filename = url[index+1:]
        thing = filename.find('?')
        if thing != -1:
            filename = filename[:thing]
        return filename

def validImage(url):
    filename = getFileName(url)
    print(url + " -> " + filename)
    for ext in valid_extensions:
        if filename.endswith(ext):
            return True
    return False


def extractURLS(post):
    any_url = []
    if validImage(post.url):
        any_url.append(post.url)
    if post.is_self:
        matches = re.findall("https?:\/\/[\w\-%\.\/\=\?\&]+",
            post.selftext)
        for x in matches:
            if validImage(getFileName(x)):
                any_url.append(x)
    return any_url

def getScams(array):
    scamResults = {}
    for x in SCAMS:
        result = x.PercentageMatch(array)
        logging.debug("{0}: {1}".format(x, result))
        if result > THRESHOLD:
            scamResults[x] = result
    return scamResults


def handleFileName(path, filename):
    text = ocr.getTextFromPath(path, filename)
    text = text.lower()
    array = re.findall(r"[\w']+", text)
    if len(sys.argv) > 1:
        logging.info(" ".join(array))
        logging.info("==============")
    return getScams(array)

def handleUrl(url):
    filename = getFileName(url)
    try:
        r = requests_retry_session(retries=5).get(url)
    except Exception as x:
        logging.error('Could not handle url:', url, x.__class__.__name__)
        print(str(x))
        try:
            e = webHook.getEmbed("Errored With Image",
                str(x), url, x.__class__.__name__)
            logging.info(str(e))
            webHook._sendWebhook(e)
        except:
            pass
        return
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
    return handleFileName(tempPath, filename)

def handlePost(post):
    global TOTAL_CHECKS, HISTORY_TOTAL, HISTORY
    SUFFIXES = {1: 'st', 2: 'nd', 3: 'rd'}
    urls = extractURLS(post)
    logging.info(str(urls))
    if len(urls) > 0:
        TOTAL_CHECKS += 1
    for url in urls:
        results = handleUrl(url) or []
        if len(results) > 0:
            text = ""
            for scam, confidence in results.items():
                if scam.Name not in HISTORY:
                    HISTORY[scam.Name] = 0
                HISTORY[scam.Name] += 1
                text += scam.Name + ": " + scam.Reason + "\r\n\r\n"
                print(scam.Name, confidence)
            HISTORY_TOTAL += 1
            if 10 <= HISTORY_TOTAL % 100 <= 20:
                suffix = 'th'
            else:
                suffix = SUFFIXES.get(HISTORY_TOTAL % 10, 'th')
            TEMPLATE = TEMPLATES[scam.Template]
            built = TEMPLATE.format(text, TOTAL_CHECKS, str(HISTORY_TOTAL) + suffix)
            if os.name != "nt" or subReddit.display_name == "mlapi":
                post.reply(built)
            webHook.sendSubmission(post, text)
            logging.info("Replied to: " + post.title)
            break
    save_history()



def loopPosts():
    for post in subReddit.new(limit=25):
        if post.name in latest_done:
            break # Since we go new -> old, don't go any further into old
        logging.info("New: " + post.title)
        saveLatest(post.name)
        handlePost(post)

def deleteBadHistory():
    for comment in reddit.user.me().comments.new(limit=10):
        if comment.score < 0:
            webHook.sendRemovedComment(comment)
            comment.delete()


load_scams()
if __name__ == "__main__":
    logging.basicConfig(filename='mlapi.log', level=logging.INFO)
    logging.getLogger().addHandler(logging.StreamHandler(sys.stdout))
    if len(sys.argv) == 2:
        path = sys.argv[1]
        if path.startswith("http"):
            print(handleUrl(path))
        else:
            fileName = getFileName(path)
            print(handleFileName(path, fileName))
        exit(0)
    setup()
    doneOnce = False
    while True:
        if not doneOnce:
            logging.info("Starting loop")
        try:
            loopPosts()
        except Exception as e:
            logging.error(e, exc_info=1)
            time.sleep(5)
        if not doneOnce:
            logging.info("Checked posts loop")
        try:
            loopInbox()
        except Exception as e:
            logging.error(e, exc_info=1)
            time.sleep(5)
        if not doneOnce:
            logging.info("Checked inbox first loop")
        try:
            deleteBadHistory()
        except Exception as e:
            logging.error(e, exc_info=1)
            time.sleep(5)
        if not doneOnce:
            logging.info("Finished loop")
            doneOnce = True



