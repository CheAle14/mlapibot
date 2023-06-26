import praw
import logging
import re
import requests
import json
import tempfile
import os, sys, time
import imgurpython

from typing import List, Union
from datetime import datetime
from praw.models import Message, Comment, Submission, Subreddit, Redditor
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry
from urllib.parse import urlparse


from mlapi.models.status import StatusAPI, StatusReporter, StatusSummary
from mlapi.models.texthighlight import TextHighlight
from mlapi.models.words import OCRImage, RedditGroup

# with open(summary.json, "r") as f:
# debug = json.load(f)
debug = None
status_reporter = StatusReporter(StatusAPI("https://discordstatus.com/api/v2", debug))


import mlapi.ocr as ocr
from mlapi.models.response_builder import ResponseBuilder
from mlapi.models.openthendelete import OpenThenDelete
from mlapi.models.scam import Scam
from mlapi.models.scam_encoder import ScamEncoder
from mlapi.webhook import WebhookSender

print(os.getcwd())
os.chdir(os.path.join(os.getcwd(), "data"))

ocr_scam_pattern = r"(?:\bhttps://)?[-A-Za-z0-9+&@#/%?=~_|!:,.;]+[-A-Za-z0-9+&@#/%=~_|]"
#discord_invite_pattern = r"https:\/\/discord\.(?:gg|com\/invites)\/([A-Za-z0-9-]{5,16})"
valid_extensions = [".png", ".jpeg", ".jpg"]

MAX_SAVE_COUNT = 250
SUFFIXES = {1: 'st', 2: 'nd', 3: 'rd'}

subReddit: Subreddit # type hint
IMGUR: imgurpython.ImgurClient = None

def load_reddit():
    global reddit, subReddit, author, testReddit
    author = "DarkOverLordCO"
    reddit = praw.Reddit("bot1", user_agent="script:mlapiOCR:v0.0.5 (by /u/" + author + ")")
    srName = "DiscordApp"
    if os.name == "nt":
        srName = "mlapi"
    subReddit = reddit.subreddit(srName)
    testReddit = reddit.subreddit("mlapi")
def load_imgur():
    global IMGUR
    try:
        with open("imgur.json") as f:
            config = json.load(f)
        IMGUR = imgurpython.ImgurClient(config["client_id"], config["client_secret"])
    except Exception as e:
        logging.error(e, exc_info=1)
        IMGUR = None
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
            upLow = "name" in scm
            name = scm["name" if upLow else "Name"]
            ocr = scm.get("ocr" if upLow else "OCR", [])
            title = scm.get("title" if upLow else "Title", [])
            body = scm.get("body" if upLow else "Body", [])
            blacklist = scm.get("blacklist" if upLow else "Blacklist", [])
            images = scm.get("images" if upLow else "Images", [])
            funcs = scm.get("functions" if upLow else "Functions", [])
            selfposts = scm.get("ignore_self_posts", False)
            report = scm.get("report", False)
            scam = Scam(name, ocr, title, body, blacklist, images, funcs, selfposts, template, report)
            SCAMS.append(scam)
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
    load_imgur()
    status_reporter.load()

    try:
        with open("webhook.txt", "r") as f:
            WEBHOOK_URL = f.read().strip()
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
                if len(latest_done) > MAX_SAVE_COUNT:
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
    if len(latest_done) > MAX_SAVE_COUNT:
        latest_done.pop(0)
    with open("save.txt", "w") as f:
        f.write("\n".join(latest_done))

def addScam(content):
    lines = content.split("\n")
    name = lines[0]
    texts = lines[1:]
    scm = Scam(name, texts, None)
    SCAMS.append(scm)
    save_scams()

def debugCheckForUrls(user : Redditor, submission : Submission):
    builder = determineScams(submission)
    msg = f"For [this submission]({submission.shortlink}), text seen was:\r\n\r\n> "
    msg += str(builder)

    user.message(subject="Debug analysis for manual response", message=msg)


def handleInboxMessage(message : Message, text : str, isAdmin : bool) -> bool:
    if text.startswith("https"):
        handlePost(message)
        return True
    return False

def handleMentionMessage(comment : Comment, text : str, isAdmin : bool) -> bool:
    split = text.split()
    if split[0] == "send":
        try:
            templateName = split[1]
        except IndexError:
            templateName = "default"
        if templateName in TEMPLATES:
            built = TEMPLATES[templateName]
            comment.reply(built)
        else:
            comment.reply(f"No template exists by name '{split[0]}'")
        if isAdmin:
            debugCheckForUrls(comment.author, comment.submission)
        return True
    elif split[0] == "stats":
        perc = int((HISTORY_TOTAL / TOTAL_CHECKS) * 100)
        comment.reply(f"I have seen a total of {TOTAL_CHECKS} submissions with {HISTORY_TOTAL} detected as scams causing me to reply, which is approximately {perc}%")
        return True
    return False

def handleUserMsg(post : Union[Message, Comment], isAdmin: bool) -> bool:
    text = post.body
    if text.startswith("/u/mlapibot "):
        text = text[len("/u/mlapibot "):]
    if isinstance(post, Message):
        return handleInboxMessage(post, text, isAdmin)
    else:
        return handleMentionMessage(post, text, isAdmin)

def loopInbox():
    unread_messages = []
    for item in reddit.inbox.unread(limit=None):
        unread_messages.append(item)
    reddit.inbox.mark_read(unread_messages)
    for x in unread_messages:
        for property, value in vars(x).items():
            print(property, ":", value)
        webHook.sendInboxMessage(x)
        logging.warning("%s: %s", x.author.name, x.body)
        if isinstance(x, Message):
            done = handleUserMsg(x, x.author.name == author)
        elif isinstance(x, Comment):
            if not x.body.startswith("/u/mlapibot"):
                continue
            done = handleUserMsg(x, x.author.name == author)
            if not done:
                x.reply("Sorry! I'm not sure what you wanted me to do.")

def getFileName(url):
    parsed = urlparse(url)
    if parsed.scheme != "https":
        return None
    path = parsed.path
    index = path.rfind('/')
    if index == -1:
        index = path.rfind('\\')
    filename = path[index+1:]
    thing = filename.find('?')
    if thing != -1:
        filename = filename[:thing]
    return filename

def validImage(url):
    filename = getFileName(url)
    if filename is None:
        return False
    print(url + " -> " + filename)
    for ext in valid_extensions:
        if filename.endswith(ext):
            return True
    return False

def extractURLSText(text: str, pattern: str) -> List[str]:
    any_url = []
    matches = re.findall(pattern,
            text)
    for x in matches:
        any_url.append(x)
    return any_url

def fixUrl(url):
    uri = urlparse(url)
    if uri.scheme != "http" and uri.scheme != "https":
        return ""
    print(uri)
    if uri.hostname == "preview.redd.it":
        url = url.replace("preview.redd.it", "i.redd.it")
    elif uri.hostname == "gyazo.com":
        url = url.replace("gyazo.com", "i.gyazo.com") + ".png"
    return url

def extractURLS(post, pattern: str):
    any_url = []
    if isinstance(post, Submission):
        if re.match(pattern, post.url) is not None:
            any_url.append(post.url)
        if post.is_self:
            any_url.extend(extractURLSText(post.selftext, pattern))
        if hasattr(post, "media_metadata"):
            for i in post.media_metadata.items():
                url = i[1]['p'][0]['u']
                url = url.split("?")[0].replace("preview", "i")
                if re.match(pattern, url) is not None:
                    any_url.append(url)
    elif isinstance(post, Message):
        any_url.extend(extractURLSText(post.body, pattern))
    elif isinstance(post, Comment):
        any_url.extend(extractURLSText(post.body, pattern))

    return [fixUrl(x) for x in any_url if x is not None]

# def getScams(array : List[str], isSelfPost, builder: ResponseBuilder) -> ResponseBuilder:
#     scamResults = {}
#     for x in SCAMS:
#         if x.IgnoreSelfPosts and isSelfPost:
#             logging.debug("Skipping {0} as self post".format(x.Name))
#             continue
#         if x.IsBlacklisted(array, builder):
#             logging.debug("Skipping {0} as blacklisted".format(x.Name))
#             continue
#         result = x.TestOCR(array, builder)
#         logging.debug("{0}: {1}".format(x, result))
#         if result > THRESHOLD:
#             scamResults[x] = result
#             builder.FormattedText = builder.TestGrounds
#             #print(builder.FormattedText)
#     builder.Add(scamResults)
#     return builder

def readFromFileName(path: str, filename: str) -> ocr.OCRImage:
    image = ocr.getTextFromPath(path, filename)
    if len(sys.argv) > 1:
        logging.info(str(image))
        logging.info("==============")
    return image

def handleUrl(url: str):
    filename = getFileName(url)
    try:
        r = requests_retry_session(retries=5).get(url)
    except Exception as x:
        logging.error('Could not handle url: {0} {1}'.format(url, x.__class__.__name__))
        print(str(x))
        try:
            e = webHook.getEmbed("Error With Image",
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
    with OpenThenDelete(tempPath, "wb") as f:
        f.write(r.content)
        return readFromFileName(tempPath, filename)

def getScamsForUrl(url : str, scams) -> ResponseBuilder:
    image = handleUrl(url)
    if image is None:
        return None

    builder = ResponseBuilder()
    builder.OCRGroups.append(image)

    scam:Scam = None
    for scam in scams:
        if scam.IsBlacklisted(image, THRESHOLD): continue

        conf = scam.TestOCR(image, THRESHOLD)
        if conf > THRESHOLD:
            logging.info(f"Seen {scam.Name} via OCR {conf*100}% of {url}")
            builder.Add({scam: conf})
        if scam.TestSubImages(image):
            logging.info(f"Seen {scam.Name} via image template")
            builder.Add({scam: 1.5})
        if scam.TestFunctions(image):
            logging.info(f"Seen {scam.Name} via functions")
            builder.Add({scam: 2})

    return builder


def getScamsInTitle(title, scams) -> ResponseBuilder:
    group = RedditGroup(title)
    builder = ResponseBuilder()
    builder.RedditGroups.append(group)
    scam: Scam = None
    for scam in scams:
        if scam.IsBlacklisted(group, THRESHOLD): continue

        conf = scam.TestTitle(group, THRESHOLD)
        if conf > THRESHOLD:
            builder.Add({scam: conf})
    return builder

def getScamsInBody(body, scams) -> ResponseBuilder:
    group = RedditGroup(body)
    builder = ResponseBuilder()
    builder.RedditGroups.append(group)
    scam: Scam = None
    for scam in scams:
        if scam.IsBlacklisted(group, THRESHOLD): continue

        conf = scam.TestBody(group, THRESHOLD)
        if conf > THRESHOLD:
            builder.Add({scam: conf})
    return builder

def removeBlacklistedScams(builder: ResponseBuilder, scams) -> ResponseBuilder:
    groups = [*builder.OCRGroups, *builder.RedditGroups]
    for group in groups:
        for scam in scams:
            if scam.IsBlacklisted(group, THRESHOLD):
                builder.Remove(scam)
    return builder

def determineScams(post: Submission) -> ResponseBuilder:
    urls = extractURLS(post, ocr_scam_pattern)
    is_selfpost = hasattr(post, "is_self") and post.is_self
    relevant_scams = []
    for scam in SCAMS:
        if is_selfpost and scam.IgnoreSelfPosts: continue
        relevant_scams.append(scam)
    ocr_urls = [x for x in urls if validImage(x)]
    builder = ResponseBuilder()
    for url in ocr_urls:
        done = getScamsForUrl(url, relevant_scams)
        if done is None: continue
        builder = builder + done


    builder += getScamsInTitle(post.title if hasattr(post, "title") else post.subject, relevant_scams)

    if hasattr(post, "selftext"):
        builder += getScamsInBody(post.selftext, relevant_scams)
    elif hasattr(post, "body"):
        builder += getScamsInBody(post.body, relevant_scams)
    return removeBlacklistedScams(builder, relevant_scams)

def checkPostForIncidentReport(post : Submission, wasBeforeStatus : bool):
    if not post.selftext: return
    if len(status_reporter.incidentsTracked) == 0: return
    if post.subreddit.display_name != "mlapi": return

    keywords = {}
    for id, inc in status_reporter.incidentsTracked.items():
        for key, v in inc.getKeywords().items():
            keywords[key] = v
    words = [x.lower() for x in post.selftext.split()] + [x.lower() for x in post.title.split()]
    match = None
    for word in words:
        if word in keywords:
            match = (word, keywords[word])
            break
    if match:
        body = "Detected a"
        body += ("n old " if wasBeforeStatus else " new ")
        body += "post which might be talking about this incident:\r\n\r\n"
        body += "[Link here](" + post.shortlink + ")\r\n\r\n"
        body += "**" + match[0] + "** matches in\r\n\r\n>" + match[1]
        (subm, created) = status_reporter.getOrCreateSubmission(testReddit)
        subm.reply(body=body)
    

def uploadToImgur(group: OCRImage, album) -> str:
    seenCopy = group.getSeenCopy()
    dir = os.path.dirname(group.path)
    filename = os.path.basename(group.path)
    seenpath = os.path.join(dir, "seen_" + filename)
    seenCopy.save(seenpath)
    scampath = os.path.join(dir, "scam_" + filename)
    scamCopy = group.getScamCopy()
    scamCopy.save(scampath)

    first =  IMGUR.upload_from_path(seenpath, {'description': 'This shows all words detected through OCR.\nColours represent confidence:\nRed = very low\nOrange = low\nBlue = moderate\nGreen = high'})
    second = IMGUR.upload_from_path(scampath, {'description': 'Red boxes are words that make up phrases previously seen. Blue boxes are standalone words not in phrases'})
    return IMGUR.make_request("POST", 'album/%s/add' % album, {"deletehashes": first["deletehash"]  +"," + second["deletehash"]})


def getImgurLink(builder: ResponseBuilder):
    if len(builder.Scams) == 0: return None
    if len(builder.OCRGroups) == 0: return None
    if IMGUR is None: return None
    try:
        album = IMGUR.create_album({'title': '/u/mlapibot OCR'})
    except Exception as e:
        logging.error(e, exc_info=1)
        return None

    try:
        for image in builder.OCRGroups:
            uploadToImgur(image, album['deletehash'])
    except Exception as e:
        logging.error(e, exc_info=1)
        try: 
            IMGUR.album_delete(album["deletehash"])
        except: pass
        return None
    
    return "https://imgur.com/a/" + album['id']
    



def handlePost(post: Union[Submission, Message, Comment], printRawTextOnPosts = False) -> ResponseBuilder:
    global TOTAL_CHECKS, HISTORY_TOTAL, HISTORY

    if post.author.id == reddit.user.me().id:
        logging.info("Ignoring post made by ourselves.")
        return None

    

    IS_POST = isinstance(post, Submission)
    DO_TEXT = post.author.name == author or \
              (not IS_POST and post.parent_id is None)
    if IS_POST and post.subreddit.name == "mlapi":
        DO_TEXT = True

    builder = determineScams(post)
    results = builder.Scams
    replied = False
    if len(results) > 0 and IS_POST:
        TOTAL_CHECKS += 1
    if len(results) > 0:
        doSkip = False
        doReport = True
        for scam, confidence in results.items():
            if scam.Name not in HISTORY:
                HISTORY[scam.Name] = 0
            HISTORY[scam.Name] += 1
            if scam.Name == "IgnorePost":
                doSkip = True
            doReport = doReport or scam.Report
            print(scam.Name, confidence, scam.Report)
        if IS_POST:
            HISTORY_TOTAL += 1
        if 10 <= HISTORY_TOTAL % 100 <= 20:
            suffix = 'th'
        else:
            suffix = SUFFIXES.get(HISTORY_TOTAL % 10, 'th')
        TEMPLATE = TEMPLATES[scam.Template]
        built = TEMPLATE.format(TOTAL_CHECKS, str(HISTORY_TOTAL) + suffix)

        imgur = getImgurLink(builder)
        if imgur is not None:
            built += f" ^[[OCR]]({imgur})"

        if DO_TEXT:
            built += "\r\n - - -"
            if doSkip:
                built += "Detected words indicating I should ignore this post, possibly legit.  "
            built += "\r\nAfter character recognition, text I saw was:\r\n\r\n> {0}\r\n".format(str(builder))
            post.reply(built)
            replied = True
        elif IS_POST and (os.name != "nt" or subReddit.display_name == "mlapi"):
            if not doSkip:
                post.reply(built)
                if doReport:
                    post.report("Appears to be a common repost")
            replied = True
            webHook.sendSubmission(post, builder.ScamText)
            logging.info("Replied to: " + post.title)
    if IS_POST:
        save_history()
        checkPostForIncidentReport(post, False)
    else:
        if not replied:
            imgur = getImgurLink(builder)
            link = "text I saw was"
            if imgur is not None:
                link = f"[{link}]({imgur})"
            post.reply(f"No scams detected; {link}:\r\n\r\n> {str(builder)}\r\n")
    return builder

def loopPosts():
    for post in subReddit.new(limit=25):
        if post.name in latest_done:
            break # Since we go new -> old, don't go any further into old
        logging.info("Post new: " + post.title)
        saveLatest(post.name)
        handlePost(post)

def deleteBadHistory():
    for comment in reddit.user.me().comments.new(limit=10):
        if comment.score < 0:
            webHook.sendRemovedComment(comment)
            comment.delete()

def handleStatusChecks():
    noPreviousSubmission = status_reporter.postId is None
    subm = status_reporter.checkStatus(testReddit, subReddit)
    if subm and noPreviousSubmission:
        logging.info("Made new status incident submission " + subm.shortlink + "; sending webhook..")
        webHook.sendStatusIncident(subm)
        # Now we should backdate to see if any previous posts were talking about this incident.
        post: Submission
        statusPostSentAt = datetime.utcfromtimestamp(int(subm.created_utc))
        for post in subReddit.new():
            if post.author.id == reddit.user.me().id: 
                continue
            sentAt = datetime.utcfromtimestamp(int(post.created_utc))
            if sentAt < statusPostSentAt:
                diff = statusPostSentAt - sentAt
                if diff.total_seconds() < (60 * 30):
                    checkPostForIncidentReport(post, True)



load_scams()

def start():
    logLevel = logging.INFO if os.name == "nt" else logging.INFO
    logging.basicConfig(
        level=logLevel,
        format="%(asctime)s [%(levelname)s] %(message)s",
        handlers=[
            logging.FileHandler("mlapi.log"),
            logging.StreamHandler(sys.stdout)
        ]
    )
    if len(sys.argv) == 2:
        path = sys.argv[1]
        if path.startswith("http"):
            image = handleUrl(path)
            image.getSeenCopy().show()
        else:
            print("That functionality has been temporarily removed")
            #fileName = getFileName(path)
            #print(handleFileName(path, fileName))
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
            logging.info("Deleted bad history.")
        try:
            handleStatusChecks()
        except Exception as e:
            logging.error(e, exc_info=1)
            time.sleep(5)

        if not doneOnce:
            logging.info("Finished loop")
            doneOnce = True



if __name__ == "__main__":
    start()

