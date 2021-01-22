import os, re
from typing import List
from json import JSONEncoder
from .response_builder import ResponseBuilder

class Scam:
    def __init__(self, name: str, texts: List[str], templateName):
        self.Name = name
        self.Texts = []
        self.Template = templateName or "default"
        for x in texts:
            self.Texts.append(x.lower())
    def  __str__(self):
        return self.Name
    def __repr__(self):
        return self.Name
    def numWordsContain(self, words: List[str], testWords: List[str], builder: ResponseBuilder) -> int:
        count = 0
        for x in testWords:
            if x in words:
                escaped = r"(?<!_)" + re.escape(x) + r"(?!_)"
                builder.TestGrounds = re.sub(escaped, "_" + x + "_", builder.TestGrounds)
                count += 1
                continue
        return count

    def findPhraseInOrder(self, words, testWords, builder: ResponseBuilder, limY = 0, limTest = 0):
        current = 0
        phraseStart = limTest
        #if self.Name == "Partner Bot":
        #    print("Looking for '" + " ".join(testWords[phraseStart:]), "' from " + str(limY) + " onwards")
        textSeen = []
        for testing in range(limTest, len(testWords)):
            for y in range(limY, len(words)):
                if testing >= len(testWords):
                    break
                word = testWords[testing]
                against = words[y]
                if against == "":
                    continue
                if word == against:
                    #if self.Name == "Discord Rewards":
                    #    print(testing, y, limY, limTest, ":", word, against)
                    current += 1
                    y += 1
                    limTest = testing + 1
                    limY = y
                    textSeen.append(word)
                    break
                elif current > 0:
                    diff = y - limY
                    if diff > 3:
                        #print("Did find", testWords[testing], "vs", words[limY-1])
                        return (current, limY + 1, phraseStart)
            if current == 0:
                # havn't found the phrase at all
                return (current, limY, phraseStart + 1)
        if current > 0 and (len(textSeen) > (len(testWords)/2)):
            sawWords = " ".join(textSeen)
            #print("Saw '" + sawWords + "' of '" + " ".join(testWords) + "'")
            escaped = r"(?<!\*\*)" + re.escape(sawWords) + r"(?!\*\*)"
            #print("REGEX: " + escaped)
            builder.TestGrounds = re.sub(escaped, "**" + sawWords + "**", builder.TestGrounds)

        return (current, limY, len(testWords))

    def phrasesInOrder(self, words: List[str], testWords: List[str], builder: ResponseBuilder) -> int:
        doneTest = 0
        limY = 0
        total = 0
        current = 0
        while doneTest < len(testWords) and limY < len(words):
            current, limY, doneTest = \
                self.findPhraseInOrder(words, testWords, builder, limY, doneTest)
            total += current
        return current

    def PercentageMatch(self, words: List[str], builder: ResponseBuilder) -> float:
        highest = 0
        high_str = None
        for testString in self.Texts:
            #print("=======BREAK==========")
            testArray = testString.split(' ')
            builder.CleanTest()
            inOrder = self.phrasesInOrder(words, testArray, builder)
            contain = self.numWordsContain(words, testArray, builder)
            total = contain + inOrder
            perc = total / (len(testArray) * 2)
            if perc > highest:
                highest = perc
                high_str = testString
        if os.name == "nt":
            print(highest, self.Name, high_str)
        return highest