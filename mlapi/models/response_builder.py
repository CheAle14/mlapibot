import os, re
from typing import List
from json import JSONEncoder

class ResponseBuilder:
    def __init__(self, threshold):
        self.Threshold = threshold
        self.ScamText = ""
        self.RecognisedText = ""
        self.FormattedText = ""
        self.TestGrounds = ""
        self.Scams = {}

    def getScamText(self):
        txt = ""
        for scam, confidence in self.Scams.items():
            txt += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))

    def Load(self, results):
        self.Scams = results
        for scam, confidence in results.items():
            self.ScamText += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))
    def Add(self, results):
        for scam, confidence in results.items():
            self.ScamText += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))
            self.Scams[scam] = confidence
    def CleanTest(self):
        self.TestGrounds = self.FormattedText
    def Remove(self, scam):
        item = None
        try:
            item = self.Scams.pop(scam)
            self.ScamText = self.getScamText()
        except:
            pass
        return item
    def __str__(self):
        return self.FormattedText