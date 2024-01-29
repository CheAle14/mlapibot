import json
import logging
import os
import sys
import tempfile
from typing import List, Union
from urllib.parse import urlparse

import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

import mlapi.ocr as ocr
from mlapi import __version__
from mlapi.models.openthendelete import OpenThenDelete
from mlapi.models.response_builder import ResponseBuilder
from mlapi.models.scam_encoder import ScamEncoder
from mlapi.models.scams import BaseScamChecker, ScamContext
from mlapi.models.scams.funcscam import FunctionScamChecker
from mlapi.models.scams.imgscam import ImgScamChecker
from mlapi.models.scams.ocrscam import OCRScamChecker
from mlapi.models.scams.textscam import TextScamChecker
from mlapi.models.words import OCRImage


class MLAPIData:
    ocr_scam_pattern = r"(?:\bhttps://)?[-A-Za-z0-9+&@#/%?=~_|!:,.;]+[-A-Za-z0-9+&@#/%=~_|]"
    #discord_invite_pattern = r"https:\/\/discord\.(?:gg|com\/invites)\/([A-Za-z0-9-]{5,16})"
    valid_extensions = [".png", ".jpeg", ".jpg"]

    MAX_SAVE_COUNT = 250
    SUFFIXES = {1: 'st', 2: 'nd', 3: 'rd'}
    THRESHOLD = 0.9

    def __init__(self, data_dir):
        self.data_dir = data_dir
        self.SCAMS: List[BaseScamChecker] = []
        print("Loading scams from", self.data_dir)

        self.load_scams()

    def _scam_object_hook(self, dct):
        if "name" not in dct:
            return dct
        try:
            type = dct.get("type", "ocr")
            if type == "text":
                return TextScamChecker.from_json(dct)
            if type == "function":
                return FunctionScamChecker.from_json(dct)
            if type == "ocr":
                return OCRScamChecker.from_json(dct)
            if type == "img":
                return ImgScamChecker.from_json(dct)
            
            raise ValueError(f"Checker type {type} is unknown")
        except Exception as e:
            print("Failed to parse", e)
            print(dct)
            raise

    def load_scams(self):
        self.SCAMS = []

        try:
            with open(os.path.join(self.data_dir, "scams.json")) as f:
                rawText = f.read()
            obj = json.loads(rawText, object_hook=self._scam_object_hook)
            for scam in obj["scams"]:
                assert isinstance(scam, BaseScamChecker)
                self.SCAMS.append(scam)
        except Exception as e:
            logging.error(e)
            print(e)
            self.SCAMS = []
            raise

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

    def getFileName(self, url):
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

    def readFromFileName(self, path: str, filename: str) -> ocr.OCRImage:
        image = ocr.getTextFromPath(path, filename)
        if len(sys.argv) > 1:
            logging.info(str(image))
            logging.info("==============")
        return image

    def download_url(self, url: str) -> Union[OCRImage, None]:
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

    def getScamsForContext(self, context: ScamContext, scams: List[BaseScamChecker]) -> ResponseBuilder:
        builder = ResponseBuilder()
        if len(context.images) > 0:
            builder.OCRGroups.extend(context.images)
        if context.title:
            builder.RedditGroups.append(context.title)
        if context.body:
            builder.RedditGroups.append(context.title)

        selected = None
        prefix = "ocr-"
        for i, scam in enumerate(scams):
            context.push(f"{prefix}{i}")
            if scam.is_blacklisted(context, self.THRESHOLD):
                continue

            result = scam.matches(context, self.THRESHOLD)
            if result >= self.THRESHOLD:
                selected = i
                builder.Add({scam: result})
        context.keep_only(prefix, selected)
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
        session.headers.update({"User-Agent": f"mlapibot v{__version__} via requests/{requests.__version__}"})
        adapter = HTTPAdapter(max_retries=retry)
        session.mount('http://', adapter)
        session.mount('https://', adapter)
        return session