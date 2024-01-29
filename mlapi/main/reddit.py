import json
import logging
import os
import re
import sys
import time
from datetime import datetime
from typing import List, Union
from urllib.parse import urlparse

import imgurpython
import praw
from praw.models import Comment, Message, Redditor, Submission, Subreddit

from mlapi.main.data import MLAPIData
from mlapi.models.response_builder import ResponseBuilder
from mlapi.models.scams import ScamContext
from mlapi.models.status import StatusAPI, StatusReporter, StatusSummary
from mlapi.models.texthighlight import TextHighlight
from mlapi.models.words import OCRImage, RedditGroup
from mlapi.webhook import WebhookSender


class MLAPIReddit(MLAPIData):
    subReddit: Subreddit # type hint
    IMGUR: imgurpython.ImgurClient = None

    def __init__(self, dir, status_debug = None):
        super().__init__(dir)
        self.status_reporter = StatusReporter(StatusAPI("https://discordstatus.com/api/v2", status_debug))
        self.load_reddit()
        self.load_imgur()
        self.status_reporter.load()

        try:
            with open(os.path.join(self.data_dir, "webhook.txt"), "r") as f:
                self.WEBHOOK_URL = f.read().strip()
        except Exception as e:
            logging.error(e)
            logging.warning("Disabling webhook sending as missing URL")
            self.WEBHOOK_URL = None

        self.webHook = WebhookSender(self.WEBHOOK_URL, self.subReddit.display_name)

        self.latest_done = []
        self.handled_posts = []
        self.handled_messages = []

        try:
            with open(os.path.join(self.data_dir, "save.txt"), "r") as f:
                for x in f:
                    self.latest_done.append(x.rstrip())
                    if len(self.latest_done) > self.MAX_SAVE_COUNT:
                        self.latest_done.pop(0)
        except Exception as e:
            logging.error(e)
            self.latest_done = []
            logging.warn("Failed to load previously handled things")

        try:
            self.load_templates()
        except Exception as e:
            logging.error(e)

        if not self.TEMPLATES:
            logging.error("Refusing to continue: Templates is empty")
            raise ValueError(self.TEMPLATES, "templates is empty")
        self.load_history()

    # def getScamsInTitle(self, title, scams) -> ResponseBuilder:
    #     group = RedditGroup(title)
    #     builder = ResponseBuilder()
    #     builder.RedditGroups.append(group)
        
    #     prefix = "title-"
    #     selected = None
    #     scam:Scam = None
    #     for i in range(len(scams)):
    #         scam = scams[i]
    #         group.push(prefix + str(i))

    #         if scam.IsBlacklisted(group, self.THRESHOLD): continue

    #         conf = scam.TestTitle(group, self.THRESHOLD)
    #         if conf > self.THRESHOLD:
    #             selected = i
    #             builder.Add({scam: conf})
    #     group.keep_only(prefix, selected)
    #     return builder

    # def getScamsInBody(self, body, scams) -> ResponseBuilder:
    #     group = RedditGroup(body)
    #     builder = ResponseBuilder()
    #     builder.RedditGroups.append(group)
        
    #     prefix = "body-"
    #     selected = None
    #     scam:Scam = None
    #     for i in range(len(scams)):
    #         scam = scams[i]
    #         group.push(prefix + str(i))

    #         if scam.IsBlacklisted(group, self.THRESHOLD): continue

    #         conf = scam.TestBody(group, self.THRESHOLD)
    #         if conf > self.THRESHOLD:
    #             selected = i
    #             builder.Add({scam: conf})
    #     group.keep_only(prefix, selected)
    #     return builder

    def load_history(self):
        self.HISTORY = {}
        self.HISTORY_TOTAL = 0
        self.TOTAL_CHECKS = 0
        try:
            with open(os.path.join(self.data_dir, "history.json"), "r") as f:
                content = f.read()
        except:
            return
        obj = json.loads(content)
        self.HISTORY = obj["history"]
        self.HISTORY_TOTAL = obj["scams"]
        self.TOTAL_CHECKS = obj["total"]

    def save_history(self):
        content = json.dumps({
            "history": self.HISTORY,
            "total": self.TOTAL_CHECKS,
            "scams": self.HISTORY_TOTAL
        })
        try:
            with open (os.path.join(self.data_dir, "history.json"), "w") as f:
                f.write(content)
        except Exception as e:
            logging.error(e)

    def load_reddit(self):
        self.author = "DarkOverLordCO"
        old_cwd = os.getcwd()
        os.chdir(self.data_dir)
        self.reddit = praw.Reddit("bot1", user_agent="script:mlapiOCR:v0.0.5 (by /u/" + self.author + ")")
        os.chdir(old_cwd)
        srName = "DiscordApp"
        if os.name == "nt":
            srName = "mlapi"
        self.subReddit = self.reddit.subreddit(srName)
        self.testReddit = self.reddit.subreddit("mlapi")
    
    def load_imgur(self):
        try:
            with open(os.path.join(self.data_dir, "imgur.json")) as f:
                config = json.load(f)
            self.IMGUR = imgurpython.ImgurClient(config["client_id"], config["client_secret"])
        except Exception as e:
            logging.error(e, exc_info=1)
            self.IMGUR = None
    
    def saveLatest(self, thingId):
        self.latest_done.append(thingId)
        if len(self.latest_done) > self.MAX_SAVE_COUNT:
            self.latest_done.pop(0)
        with open(os.path.join(self.data_dir, "save.txt"), "w") as f:
            f.write("\n".join(self.latest_done))

    def debugCheckForUrls(self, user : Redditor, submission : Submission):
        builder = self.determineScams(submission)
        msg = f"For [this submission]({submission.shortlink}), text seen was:\r\n\r\n> "
        msg += str(builder)

        user.message(subject="Debug analysis for manual response", message=msg)


    def handleInboxMessage(self, message : Message, text : str, isAdmin : bool) -> bool:
        if text.startswith("https"):
            self.handlePost(message)
            return True
        return False

    def handleMentionMessage(self, comment : Comment, text : str, isAdmin : bool) -> bool:
        split = text.split()
        if split[0] == "send":
            try:
                templateName = split[1]
            except IndexError:
                templateName = "default"
            if templateName in self.TEMPLATES:
                built = self.TEMPLATES[templateName]
                comment.reply(built)
            else:
                comment.reply(f"No template exists by name '{split[0]}'")
            if isAdmin:
                self.debugCheckForUrls(comment.author, comment.submission)
            return True
        elif split[0] == "stats":
            perc = int((self.HISTORY_TOTAL / self.TOTAL_CHECKS) * 100)
            comment.reply(f"I have seen a total of {self.TOTAL_CHECKS} submissions with {self.HISTORY_TOTAL} detected as scams causing me to reply, which is approximately {perc}%")
            return True
        return False

    def handleUserMsg(self, post : Union[Message, Comment], isAdmin: bool) -> bool:
        text = post.body
        if text.startswith("/u/mlapibot "):
            text = text[len("/u/mlapibot "):]
        if isinstance(post, Message):
            return self.handleInboxMessage(post, text, isAdmin)
        else:
            return self.handleMentionMessage(post, text, isAdmin)

    def loopInbox(self, ):
        unread_messages = []
        for item in self.reddit.inbox.unread(limit=None):
            unread_messages.append(item)
        self.reddit.inbox.mark_read(unread_messages)
        for x in unread_messages:
            for property, value in vars(x).items():
                print(property, ":", value)
            self.webHook.sendInboxMessage(x)
            logging.warning("%s: %s", x.author.name, x.body)
            if isinstance(x, Message):
                done = self.handleUserMsg(x, x.author.name == self.author)
            elif isinstance(x, Comment):
                if not x.body.startswith("/u/mlapibot"):
                    continue
                done = self.handleUserMsg(x, x.author.name == self.author)
                if not done:
                    x.reply("Sorry! I'm not sure what you wanted me to do.")

    def validImage(self, url):
        filename = self.getFileName(url)
        if filename is None:
            return False
        print(url + " -> " + filename)
        for ext in self.valid_extensions:
            if filename.endswith(ext):
                return True
        return False

    def extractURLSText(self, text: str, pattern: str) -> List[str]:
        any_url = []
        matches = re.findall(pattern,
                text)
        for x in matches:
            any_url.append(x)
        return any_url

    def fixUrl(self, url):
        uri = urlparse(url)
        if uri.scheme != "http" and uri.scheme != "https":
            return ""
        print(uri)
        if uri.hostname == "preview.redd.it":
            url = url.replace("preview.redd.it", "i.redd.it")
        elif uri.hostname == "gyazo.com":
            url = url.replace("gyazo.com", "i.gyazo.com") + ".png"
        return url

    def extractURLS(self, post, pattern: str):
        any_url = []
        if isinstance(post, Submission):
            if re.match(pattern, post.url) is not None:
                any_url.append(post.url)
            if post.is_self:
                any_url.extend(self.extractURLSText(post.selftext, pattern))
            if hasattr(post, "gallery_data"):
                for item in post.gallery_data["items"]:
                    media_id = item["media_id"]
                    media = post.media_metadata[media_id]
                    if media["e"] == "Image":
                        url = media["s"]["u"]
                        url = url.split("?")[0].replace("preview", "i")
                        any_url.append(url)
        elif isinstance(post, Message):
            any_url.extend(self.extractURLSText(post.body, pattern))
        elif isinstance(post, Comment):
            any_url.extend(self.extractURLSText(post.body, pattern))

        return [self.fixUrl(x) for x in any_url if x is not None]


    def determineScams(self, post: Submission) -> ResponseBuilder:
        urls = self.extractURLS(post, self.ocr_scam_pattern)
        is_selfpost = hasattr(post, "is_self") and post.is_self
        relevant_scams = []
        for scam in self.SCAMS:
            if is_selfpost and scam.ignore_self_posts: continue
            relevant_scams.append(scam)
        ocr_urls = [x for x in urls if self.validImage(x)]

        ocr_images = [self.download_url(url) for url in ocr_urls]
        ocr_images = [x for x in ocr_images if x is not None]

        title = post.title if hasattr(post, "title") else post.subject
        body = post.selftext if hasattr(post, "selftext") else post.body

        with ScamContext(self, post, title, body, ocr_images) as context:
            builder = self.getScamsForContext(context, relevant_scams)
            return self.removeBlacklistedScams(builder, relevant_scams)

    def checkPostForIncidentReport(self, post : Submission, wasBeforeStatus : bool):
        if not post.selftext: return

        for id, inc in self.status_reporter.incidentsTracked.items():
            nl, keywords = inc.getKeywords()
            words = [x.lower() for x in post.selftext.split()] + [x.lower() for x in post.title.split()]
            match = None
            for word in words:
                if word in keywords:
                    match = word
                    break
            if match:
                body = "Detected a"
                body += ("n old " if wasBeforeStatus else " new ")
                body += "post which might be talking about this incident:\r\n\r\n"
                body += "[Link here](" + post.shortlink + ")\r\n\r\n"
                body += "**" + match + "** matches incident keywords."
                (subm, created) = self.status_reporter.getOrCreateSubmission(self.testReddit)
                subm.reply(body=body)
        

    def uploadToImgur(self, group: OCRImage, album) -> str:
        seenCopy = group.getSeenCopy()
        dir = os.path.dirname(group.path)
        filename = os.path.basename(group.path)
        seenpath = os.path.join(dir, "seen_" + filename)
        seenCopy.save(seenpath)
        scampath = os.path.join(dir, "scam_" + filename)
        scamCopy = group.getScamCopy()
        scamCopy.save(scampath)

        first =  self.IMGUR.upload_from_path(seenpath, {'description': 'This shows all words detected through OCR.\nColours represent confidence:\nRed = very low\nOrange = low\nBlue = moderate\nGreen = high'})
        second = self.IMGUR.upload_from_path(scampath, {'description': 'This shows the words that actually triggered a response. Colors represent their similarity to the trigger word.'})
        return self.IMGUR.make_request("POST", 'album/%s/add' % album, {"deletehashes": first["deletehash"]  +"," + second["deletehash"]})


    def getImgurLink(self, builder: ResponseBuilder):
        if len(builder.OCRGroups) == 0: return None
        if self.IMGUR is None: return None
        try:
            album = self.IMGUR.create_album({'title': '/u/mlapibot OCR'})
        except Exception as e:
            logging.error(e, exc_info=1)
            return None

        try:
            for image in builder.OCRGroups:
                self.uploadToImgur(image, album['deletehash'])
        except Exception as e:
            logging.error(e, exc_info=1)
            try: 
                self.IMGUR.album_delete(album["deletehash"])
            except: pass
            return None
        
        return "https://imgur.com/a/" + album['id']
        



    def handlePost(self, post: Union[Submission, Message, Comment], printRawTextOnPosts = False) -> ResponseBuilder:
        if post.author.id == self.reddit.user.me().id:
            logging.info("Ignoring post made by ourselves.")
            return None

        

        IS_POST = isinstance(post, Submission)
        DO_TEXT = post.author.name == self.author or \
                (not IS_POST and post.parent_id is None)
        if IS_POST and post.subreddit.name == "mlapi":
            DO_TEXT = True

        builder = self.determineScams(post)
        results = builder.Scams
        replied = False
        if len(results) > 0 and IS_POST:
            self.TOTAL_CHECKS += 1
        if len(results) > 0:
            doSkip = False
            doReport = True
            for scam, confidence in results.items():
                if scam.name not in self.HISTORY:
                    self.HISTORY[scam.name] = 0
                self.HISTORY[scam.name] += 1
                if scam.name == "IgnorePost":
                    doSkip = True
                doReport = doReport or scam.Report
                print(scam.name, confidence, scam.Report)
            if IS_POST:
                self.HISTORY_TOTAL += 1
            if 10 <= self.HISTORY_TOTAL % 100 <= 20:
                suffix = 'th'
            else:
                suffix = self.SUFFIXES.get(self.HISTORY_TOTAL % 10, 'th')
            TEMPLATE = self.TEMPLATES[scam.Template]
            built = TEMPLATE.format(self.TOTAL_CHECKS, str(self.HISTORY_TOTAL) + suffix)


            if DO_TEXT:
                imgur = self.getImgurLink(builder)
                if imgur is not None:
                    built += f" ^[[OCR]]({imgur})"
                built += "\r\n - - -"
                if doSkip:
                    built += "Detected words indicating I should ignore this post, possibly legit.  "
                built += "\r\nAfter character recognition, text I saw was:\r\n\r\n> {0}\r\n".format(str(builder))
                post.reply(built)
                replied = True
            elif IS_POST and (os.name != "nt" or self.subReddit.display_name == "mlapi"):
                imgur = self.getImgurLink(builder)
                if imgur is not None:
                    built += f" ^[[OCR]]({imgur})"
                if not doSkip:
                    post.reply(built)
                    if doReport:
                        post.report("Appears to be a common repost")
                replied = True
                self.webHook.sendSubmission(post, builder.ScamText + (f"\n[OCR]({imgur})" if imgur else ''))
                logging.info("Replied to: " + post.title)
        if IS_POST:
            self.save_history()
            self.checkPostForIncidentReport(post, False)
        else:
            if not replied:
                imgur = self.getImgurLink(builder)
                link = "text I saw was"
                if imgur is not None:
                    link = f"[{link}]({imgur})"
                post.reply(f"No scams detected; {link}:\r\n\r\n> {str(builder)}\r\n")
        return builder

    def loopPosts(self):
        for post in self.subReddit.new(limit=25):
            if post.name in self.latest_done:
                break # Since we go new -> old, don't go any further into old
            logging.info("Post new: " + post.title)
            self.saveLatest(post.name)
            self.handlePost(post)

    def deleteBadHistory(self):
        for comment in self.reddit.user.me().comments.new(limit=10):
            if comment.score < 0:
                self.webHook.sendRemovedComment(comment)
                comment.delete()

    def handleStatusChecks(self):
        noPreviousSubmission = len(self.status_reporter.posts) == 0
        subm = self.status_reporter.checkStatus(self.testReddit, self.subReddit)
        if subm and noPreviousSubmission:
            logging.info("Made new status incident submission " + subm.shortlink + "; sending webhook..")
            self.webHook.sendStatusIncident(subm)
            # Now we should backdate to see if any previous posts were talking about this incident.
            post: Submission
            statusPostSentAt = datetime.utcfromtimestamp(int(subm.created_utc))
            for post in self.subReddit.new():
                if post.author.id == self.reddit.user.me().id: 
                    continue
                sentAt = datetime.utcfromtimestamp(int(post.created_utc))
                if sentAt < statusPostSentAt:
                    diff = statusPostSentAt - sentAt
                    if diff.total_seconds() < (60 * 30):
                        self.checkPostForIncidentReport(post, True)

    def run_forever(self):
        logLevel = logging.INFO if os.name == "nt" else logging.INFO
        logging.basicConfig(
            level=logLevel,
            format="%(asctime)s [%(levelname)s] %(message)s",
            handlers=[
                logging.FileHandler("mlapi.log"),
                logging.StreamHandler(sys.stdout)
            ]
        )
        doneOnce = False
        while True:
            if not doneOnce:
                logging.info("Starting loop")
            try:
                self.loopPosts()
            except Exception as e:
                logging.error(e, exc_info=1)
                time.sleep(5)
            time.sleep(1)
            if not doneOnce:
                logging.info("Checked posts loop")
            try:
                self.loopInbox()
            except Exception as e:
                logging.error(e, exc_info=1)
                time.sleep(5)
            time.sleep(1)
            if not doneOnce:
                logging.info("Checked inbox first loop")
            try:
                self.deleteBadHistory()
            except Exception as e:
                logging.error(e, exc_info=1)
                time.sleep(5)
            time.sleep(1)

            if not doneOnce:
                logging.info("Deleted bad history.")
            try:
                self.handleStatusChecks()
            except Exception as e:
                logging.error(e, exc_info=1)
                time.sleep(5)

            if not doneOnce:
                logging.info("Finished loop")
                doneOnce = True

if __name__ == "__main__":
    client = MLAPIReddit(os.path.join(os.getcwd(), "data"))
    client.run_forever()

