import os, re
from typing import List
from json import JSONEncoder
from .response_builder import ResponseBuilder
from mlapi.ocr import checkForSubImage
from glob import glob

class Scam:
    def __init__(self, name: str, ocr: List[str], title: List[str],
                                    body: List[str],
                                    blackList: List[str],
                                    images: List[str],
                                    ignoreSelfPosts: bool,
                                    templateName : str,
                                    report : bool):
        self.Name = name
        self.OCR = ocr
        self.Title = title or []
        self.Body = body or []
        self.Blacklist = blackList or []
        self.Images = images or []
        self.IgnoreSelfPosts = ignoreSelfPosts or False
        self.Template = templateName or "default"
        self.Report = report

        self.__dbg = False #name == "Free Nitro/Boost"
    def  __str__(self):
        return self.Name
    def __repr__(self):
        return self.Name
    def numWordsContain(self, words: List[str], testWords: List[str], builder: ResponseBuilder) -> int:
        count = 0
        for x in testWords:
            try:
                idx = words.index(x)

                builder.Highlight.setItalic(idx)
                count += 1
                continue
            except ValueError: pass
        return count

    def newInOrder(self, detectedWords : List[str], testingWords : List[str], builder : ResponseBuilder):
        detectedIndex = 0
        testingIndex = 0
        numWordsSeen = 0

        consecutiveStartedAt = None

        while detectedIndex < len(detectedWords) and testingIndex < len(testingWords):
            if detectedWords[detectedIndex] == testingWords[testingIndex]:
                numWordsSeen += 1
                testingIndex += 1
                if consecutiveStartedAt is None:
                    consecutiveStartedAt = detectedIndex
            elif consecutiveStartedAt is not None:
                # not the next word and we've broken the streak.
                builder.Highlight.autowrap(consecutiveStartedAt, detectedIndex - consecutiveStartedAt)
                consecutiveStartedAt = None
            detectedIndex += 1
        if consecutiveStartedAt is not None:
            builder.Highlight.autowrap(consecutiveStartedAt, min(detectedIndex, testingIndex))
        return numWordsSeen


    def findPhraseInOrder(self, words, testWords, builder: ResponseBuilder, limY = 0, limTest = 0):
        numWordsSeen = 0
        phraseStart = limTest
        if self.__dbg:
            print("Looking for '" + " ".join(testWords[phraseStart:]), "' from " + str(limY) + " onwards")
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
                    if self.__dbg:
                        print(testing, y, limY, limTest, ":", word, against)
                    numWordsSeen += 1
                    builder.Highlight.wrapword(y, 1, "|", "|")
                    y += 1
                    limTest = testing + 1
                    limY = y
                    textSeen.append(word)
                    break
                elif numWordsSeen > 0:
                    diff = y - limY
                    if diff > 3:
                        if self.__dbg:
                            print("Did find", testWords[testing], "vs", words[limY-1])
                        return (numWordsSeen, limY + 1, phraseStart)
            if numWordsSeen == 0:
                # havn't found the phrase at all
                return (numWordsSeen, limY, phraseStart + 1)
        if numWordsSeen > 0 and (len(textSeen) > (len(testWords)/2)):
            sawWords = " ".join(textSeen)

            escaped = r"(?<!\*\*)" + re.escape(sawWords) + r"(?!\*\*)"
            if self.__dbg:
                print("Saw '" + sawWords + "' of '" + " ".join(testWords) + "'")
                print("REGEX: " + escaped)
            builder.TestGrounds = re.sub(escaped, "**" + sawWords + "**", builder.TestGrounds)

        return (numWordsSeen, limY, len(testWords))

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

    def TestItem(self, wordsPost: List[str], textsJson: List[str],
                             builder: ResponseBuilder) -> float:
        highest = 0
        high_str = None
        for testString in textsJson:
            if self.__dbg:
                print("=======BREAK:  ")
            testArray = testString.split(' ')
            builder.CleanTest()
            inOrder = self.newInOrder(wordsPost, testArray, builder) # self.phrasesInOrder(wordsPost, testArray, builder)
            contain = self.numWordsContain(wordsPost, testArray, builder)
            total = contain + inOrder
            perc = total / (len(testArray) * 2)
            if perc > highest:
                highest = perc
                high_str = testString
            if perc > builder.Threshold:
                break # no need to continue any further if we've found at least one
        if self.__dbg:
            print(str(int(highest * 100)).zfill(2) + "%", self.Name, high_str)
        return highest

    def IsBlacklisted(self, words: List[str], builder: ResponseBuilder) -> bool:
        return self.TestItem(words, self.Blacklist, builder) > 0.9

    def TestTitle(self, words: List[str], builder: ResponseBuilder) -> float:
        return self.TestItem(words, self.Title, builder)

    def TestBody(self, words: List[str], builder: ResponseBuilder) -> float:
        return self.TestItem(words, self.Body, builder)

    def TestOCR(self, words: List[str], builder: ResponseBuilder) -> float:
        return self.TestItem(words, self.OCR, builder)

    def TestSubImages(self, imageFilePath, builder: ResponseBuilder) -> bool:
        if len(self.Images) == 0: return False
        for imgPattern in self.Images:
            imgPath = os.path.join("images", imgPattern)
            imgNames = None
            if '*' in imgPattern:
                imgNames = glob(imgPath)
            else:
                imgNames = [imgPath]
            for imgName in imgNames:
                outPath = os.path.join(os.path.dirname(imageFilePath), "result_" + os.path.basename(imgName))
                if checkForSubImage(imageFilePath, imgName, outPath):
                    return True
        return False

