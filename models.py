import os
from typing import List
from json import JSONEncoder
class Scam:
    def __init__(self, name: str, reason: str, texts: List[str]):
        self.Name = name
        self.Reason = reason
        self.Texts = []
        for x in texts:
            self.Texts.append(x.lower())
    def  __str__(self):
        return self.Name
    def __repr__(self):
        return self.Name
    def numWordsContain(self, words: List[str], testWords: List[str]) -> int:
        count = 0
        for x in testWords:
            if x in words:
                count += 1
                continue
        return count

    def phrasesInOrder(self, words: List[str], testWords: List[str]) -> int:
        current = 0
        for testing in range(len(testWords)):
            for y in range(len(words)):
                if testing >= len(testWords):
                    break
                word = testWords[testing]
                against = words[y]
                if against == "":
                    continue
                if word == against:
                    current += 1
                    testing += 1
                    break
        return current

    def PercentageMatch(self, words: List[str]) -> float:
        highest = 0
        high_str = None
        for testString in self.Texts:
            testArray = testString.split(' ')
            contain = self.numWordsContain(words, testArray)
            inOrder = self.phrasesInOrder(words, testArray)
            total = contain + inOrder
            perc = total / (len(testArray) * 2)
            if perc > highest:
                highest = perc
                high_str = testString
        if os.name == "nt":
            print(highest, self.Name, high_str)
        return highest
class ScamEncoder(JSONEncoder):
        def default(self, o):
            return o.__dict__
