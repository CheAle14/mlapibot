import mlapi.ocr as ocr
from mlapi.models.openthendelete import OpenThenDelete
from mlapi.models.response_builder import ResponseBuilder
from mlapi.models.scam import Scam
from mlapi.models.scam_encoder import ScamEncoder
from mlapi.models.words import OCRImage


import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry


import json
import logging
import os
import sys
import tempfile


class MLAPIData:
    ocr_scam_pattern = r"(?:\bhttps://)?[-A-Za-z0-9+&@#/%?=~_|!:,.;]+[-A-Za-z0-9+&@#/%=~_|]"
    #discord_invite_pattern = r"https:\/\/discord\.(?:gg|com\/invites)\/([A-Za-z0-9-]{5,16})"
    valid_extensions = [".png", ".jpeg", ".jpg"]

    MAX_SAVE_COUNT = 250
    SUFFIXES = {1: 'st', 2: 'nd', 3: 'rd'}
    THRESHOLD = 0.9

    def __init__(self, data_dir):
        self.data_dir = data_dir

        self.load_scams()

    def load_scams(self):
        self.SCAMS = []

        try:
            with open(os.path.join(self.data_dir, "scams.json")) as f:
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
                self.SCAMS.append(scam)
        except Exception as e:
            logging.error(e)
            print(e)
            self.SCAMS = []

        if len(self.SCAMS) == 0:
            raise ValueError(self.SCAMS)

    def save_scams(self):
        try:
            content = json.dumps({"scams": self.SCAMS}, indent=4, cls=ScamEncoder)
            with open(os.path.join(self.data_dir, "scams.json"), "w") as f:
                f.write(content)
        except Exception as e:
            logging.error(e)

    def load_templates(self):
        self.TEMPLATES = {}
        files = os.listdir(os.path.join(self.data_dir, "templates"))
        for x in files:
            if x.endswith(".md"):
                name = x[:-3]
                with open(os.path.join(self.data_dir, "templates", x), "r") as f:
                    self.TEMPLATES[name] = f.read()

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

    def readFromFileName(self, path: str, filename: str) -> ocr.OCRImage:
        image = ocr.getTextFromPath(path, filename)
        if len(sys.argv) > 1:
            logging.info(str(image))
            logging.info("==============")
        return image

    def handleUrl(self, url: str):
        filename = self.getFileName(url)
        try:
            r = self.requests_retry_session(retries=5).get(url)
        except Exception as x:
            logging.error('Could not handle url: {0} {1}'.format(url, x.__class__.__name__))
            print(str(x))
            try:
                e = self.webHook.getEmbed("Error With Image",
                    str(x), url, x.__class__.__name__)
                logging.info(str(e))
                self.webHook._sendWebhook(e)
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
        return self.readFromFileName(tempPath, filename)

    def getScamsForImage(self, image: OCRImage, scams) -> ResponseBuilder:
        builder = ResponseBuilder()
        builder.OCRGroups.append(image)

        prefix = "ocr-"
        selected = None
        scam:Scam = None
        for i in range(len(scams)):
            scam = scams[i]
            image.push(prefix + str(i))
            if scam.IsBlacklisted(image, self.THRESHOLD): continue

            conf = scam.TestOCR(image, self.THRESHOLD)
            if conf > self.THRESHOLD:
                selected = i
                logging.info(f"Seen {scam.Name} via OCR {conf*100}%")
                builder.Add({scam: conf})
            if scam.TestSubImages(image):
                logging.info(f"Seen {scam.Name} via image template")
                builder.Add({scam: 1.5})
            if scam.TestFunctions(image):
                logging.info(f"Seen {scam.Name} via functions")
                builder.Add({scam: 2})
        image.keep_only(prefix, selected)
        return builder

    def getScamsForUrl(self, url : str, scams) -> ResponseBuilder:
        image = self.handleUrl(url)
        if image is None:
            return None
        with image:
            return self.getScamsForImage(image, scams)


    def removeBlacklistedScams(self, builder: ResponseBuilder, scams) -> ResponseBuilder:
        groups = [*builder.OCRGroups, *builder.RedditGroups]
        for group in groups:
            for scam in scams:
                if scam.IsBlacklisted(group, self.THRESHOLD):
                    builder.Remove(scam)
        return builder

    def requests_retry_session(self,
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