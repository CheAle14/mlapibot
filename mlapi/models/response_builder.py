import os, re
from typing import List
from json import JSONEncoder
from mlapi.models.texthighlight import TextHighlight

class ResponseBuilder:
    def __init__(self):
        self.ScamText = ""
        self.Scams = {}
        # self.Highlight = None

        self.OCRGroups = []
        self.RedditGroups = []
    def __add__(self, other):
        combined = ResponseBuilder()
        combined.Add(self.Scams)
        combined.Add(other.Scams)
        combined.OCRGroups.extend(self.OCRGroups)
        combined.OCRGroups.extend(other.OCRGroups)
        combined.RedditGroups.extend(self.RedditGroups)
        combined.RedditGroups.extend(other.RedditGroups)
        combined.ScamText = combined.getScamText()
        return combined

    def getScamText(self):
        txt = ""
        for scam, confidence in self.Scams.items():
            txt += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))
        return txt

    def Load(self, results):
        self.Scams = results
        self.ScamText = self.getScamText()
    def Add(self, results):
        for scam, confidence in results.items():
            self.ScamText += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))
            self.Scams[scam] = confidence
    def Remove(self, scam):
        item = None
        try:
            item = self.Scams.pop(scam)
            self.ScamText = self.getScamText()
        except:
            pass
        return item
    def __str__(self):
        s = []
        if len(self.OCRGroups) > 0:
            s.append("## OCR")
            for ocr in self.OCRGroups:
                s.append(str(ocr))
        if len(self.RedditGroups) > 0:
            s.append("## Post text")
            for red in self.RedditGroups:
                s.append(str(red))
        return "  \n> ".join(s)