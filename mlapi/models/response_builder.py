import os, re
from typing import List
from json import JSONEncoder

class ResponseBuilder:
    def __init__(self):
        self.ScamText = ""
        self.RecognisedText = ""
        self.FormattedText = ""
        self.TestGrounds = ""
        self.Scams = {}
    def Load(self, results):
        self.Scams = results
        for scam, confidence in results.items():
            self.ScamText += "{0}: {1}%  \r\n".format(scam.Name, round(confidence * 100))
    def CleanTest(self):
        self.TestGrounds = self.FormattedText
    def __str__(self):
        return self.FormattedText