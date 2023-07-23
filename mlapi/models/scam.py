import os, re
from typing import List, Tuple
from json import JSONEncoder
from .response_builder import ResponseBuilder
from mlapi.ocr import checkForSubImage, FUNCTIONS
from mlapi.models.words import BaseGroup, BaseWord, OCRImage
from glob import glob
from strsimpy.weighted_levenshtein import WeightedLevenshtein

def ins_fn(c): return 1.0
def del_fn(c): return 1.0
def sub_fn(char_a, char_b):
    return 1.0

comparer = WeightedLevenshtein(substitution_cost_fn=sub_fn, insertion_cost_fn=ins_fn, deletion_cost_fn=del_fn)




class Scam:
    def __init__(self, name: str, ocr: List[str], title: List[str],
                                    body: List[str],
                                    blackList: List[str],
                                    images: List[str],
                                    functions: List[str],
                                    ignoreSelfPosts: bool,
                                    templateName : str,
                                    report : bool):
        self.Name = name
        self.OCR = ocr
        self.Title = title or []
        self.Body = body or []
        self.Blacklist = blackList or []
        self.Images = images or []
        self.Functions = functions or []
        self.IgnoreSelfPosts = ignoreSelfPosts or False
        self.Template = templateName or "default"
        self.Report = report

        self.__dbg = False #name == "Free Nitro/Boost"
    def  __str__(self):
        return self.Name
    def __repr__(self):
        return self.Name
    def numWordsContain(self, words: List[BaseWord], testWords: List[str]) -> float:
        count = 0
        for word in words:
            if word.text in testWords:
                count += 1
                word.present = True
        percOfTest = count / len(testWords)
        percOfWords = count / len(words)
        if percOfWords < 0.25:
            # e.g. if 0.2,
            # perc * (1 - (0.25 - 0.2))
            # perc * (1 - 0.05)
            # perc * 0.95
            # apply 1% deduction for every pp below 25% of seen words
            ppBelow = (0.25 - percOfWords)
            percOfTest *= (1 - ppBelow)
        return percOfTest
        

    def newInOrder(self, words: List[BaseWord], testingWords : List[str]) -> float:
        detectedIndex = 0
        testingIndex = 0
        numWordsSeen = 0

        consecutiveStartedAt = None

        while detectedIndex < len(words) and testingIndex < len(testingWords):
            if words[detectedIndex].text == testingWords[testingIndex]:
                numWordsSeen += 1
                testingIndex += 1
                if consecutiveStartedAt is None:
                    consecutiveStartedAt = detectedIndex
            elif consecutiveStartedAt is not None:
                # not the next word and we've broken the streak.
                for word in words[consecutiveStartedAt:detectedIndex]:
                    word.consecutive = True
                consecutiveStartedAt = None
            detectedIndex += 1
        if consecutiveStartedAt is not None:
            for word in words[consecutiveStartedAt:min(detectedIndex, testingIndex)]:
                word.consecutive = True
        return numWordsSeen / len(testingWords)
    

    def is_similar_word(self, wordA, wordB, distance = None):
        if distance is None:
            distance = comparer.distance(wordA, wordB)
        return distance <= 2 and (distance / max(len(wordA), len(wordB)) < 0.5)

    def leven_distance(self, startAt: int, testStartAt: int, words: List[BaseWord], testing: List[str], _depth = 0) -> Tuple[float, float, float]:
        overallDistance = 0
        maximumPossibleDistance = 0

        tentativeCon = []


        current = startAt
        testCurrent = testStartAt
        while current < len(words) and testCurrent < len(testing):
            distance = comparer.distance(words[current].text, testing[testCurrent])
            if self.is_similar_word(words[current].text, testing[testCurrent], distance):
                words[current].seen_distance = distance
                tentativeCon.append(current)
                overallDistance += distance
                maximumPossibleDistance += max(len(words[current].text), len(testing[testCurrent]))
                testCurrent += 1
            else:
                #print("    ", " " * _depth, self.Name, "cannot find", testCurrent, testing[testCurrent], "at", current, words[current])
                # we are missing a word.
                # two methods we consider:
                # 1) pretending the test word doesn't exist and attempting to find the next test word
                # 2) pretending the group word doesn't exist and attempting to find the same test word within the next few words
                
                # so we'll start with (1):
                best_next_test = (1000, None)
                for tryIndex in range(testCurrent+1, min(testCurrent+1 + 5, len(testing))):
                    distance = comparer.distance(words[current].text, testing[tryIndex])
                    if self.is_similar_word(words[current].text, testing[tryIndex], distance):
                        best_next_test = (distance, tryIndex)
                
                best_next_group = (1001, None)
                for tryIndex in range(current+1, min(current+1 + 5, len(words))):
                    distance = comparer.distance(words[tryIndex].text, testing[testCurrent])
                    if self.is_similar_word(words[tryIndex].text, testing[testCurrent], distance):
                        best_next_group = (distance, tryIndex)
                
                if best_next_test[1] is None and best_next_group[1] is None:
                    # hidden achievement (3): unable to find either next test or next group.
                    # so our consecutive searching ends here.
                    # calculate all the remaining missing distance
                    remaining_test_sum = sum([len(x) for x in testing[testCurrent:]])
                    remaining_group_sum = 0 # sum([len(x.text) for x in words[current:]])
                    distance = max(remaining_test_sum, remaining_group_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    #print("    ", " " * _depth, self.Name, "unable to relock", remaining_test_sum, remaining_group_sum)
                    break
                elif best_next_group[1] is None or best_next_test[0] < best_next_group[0]:
                    # scenario (1): we've found the test word later on, maybe.
                    skipped_test_sum = sum([len(x) for x in testing[testCurrent:best_next_test[1]]])
                    distance = max(distance, skipped_test_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    testCurrent = best_next_test[1] + 1
                    current += 1

                    #print("    ", " " * _depth, self.Name, "found text word again", best_next_test)
                    continue
                else:
                    # scenario (2):
                    skipped_group_sum = sum([len(x.text) for x in words[current:best_next_group[1]]])
                    distance = max(distance, skipped_group_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    current = best_next_group[1] + 1
                    testCurrent += 1
                    #print("    ", " " * _depth, self.Name, "found next current", best_next_group)
                    continue


            current += 1

        if maximumPossibleDistance == 0:
            #print("    ", " "*_depth, self.Name, "found nothing")
            return (0, 0, 0)
        #print("    ", " "*_depth, self.Name, "tentative:", tentativeCon)
        if len(tentativeCon) > max(2, len(testing) * 0.1):
            for idx in tentativeCon:
                words[idx].consecutive = True
        #print("    ", " "*_depth, self.Name, (startAt, testStartAt),"->", (current, testCurrent), "got", overallDistance, "of", maximumPossibleDistance)
        return (1 - (overallDistance / maximumPossibleDistance), overallDistance, maximumPossibleDistance)
            
    def length(self, array) -> int:
        l = len(array)
        for word in array:
            if isinstance(word, str):
                l += len(word)
            else:
                l += len(word.text)
        return l
        
    def best_leven_distance(self, group: BaseGroup, words: List[BaseWord], testing: List[str], threshold: float) -> float:
        starts = []
        for i in range(len(words)):
            remain = len(words) - i - 1
            if remain < len(testing): break # not enough words left to find the test string
            # just in case the first word(s) are not seen, try and find the 
            # first couple words in the string, and add that in
            for start in range(min(len(testing), 5)):
                remain = len(testing) - start - 1
                if (remain / len(testing)) < 0.5: 
                    # not enough left to test
                    break

                # compare each word to try and find a start point
                if self.is_similar_word(words[i].text, testing[start]):
                    starts.append((i, start))
        #print("    ", self.Name, "found starts:", starts)
        best = None
        bestIdx = None
        prefix = self.Name + "-leven-"
        for i in range(len(starts)):
            pair = starts[i]
            group.push(prefix + str(i))
            score, _, _ = self.leven_distance(pair[0], pair[1], words, testing)
            #print(self.Name, pair, "normalised:", score)
            if best is None or score > best:
                best = score
                bestIdx = i
        group.keep_only(prefix, bestIdx)
        return best or 0

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

    def TestItem(self, group : BaseGroup, textsJson: List[str],
                             threshold: float) -> float:
        if len(textsJson) == 0: return 0
        highest = 0
        highestIndex = None
        high_str = None
        #group.dump()
        words = [word for word in group.words if word.conf >= 70]
        #print("Seen:", [f"{i}:\"{words[i]}\"" for i in range(len(words))])
        prefix = self.Name + "-scam-"
        for i in range(len(textsJson)):
            testString = textsJson[i]
            if self.__dbg:
                print("=======BREAK:  ")
            testArray = testString.split(' ')
            #print(self.Name, "searching:", testString)
            
            group.push(prefix + str(i))
            perc = self.best_leven_distance(group, words, testArray, threshold)
            #print(self.Name, "best =", perc)

            if perc > highest:
                highest = perc
                high_str = testString
                if perc > threshold:
                    highestIndex = i
        if self.__dbg:
            print(str(int(highest * 100)).zfill(2) + "%", self.Name, high_str)
        group.keep_only(prefix, highestIndex)
        return highest

    def IsBlacklisted(self, group: BaseGroup, threshold: float) -> bool:
        return self.TestItem(group, self.Blacklist, threshold) > threshold

    def TestTitle(self, group: BaseGroup, threshold: float) -> float:
        return self.TestItem(group, self.Title, threshold)

    def TestBody(self, group: BaseGroup, threshold: float) -> float:
        return self.TestItem(group, self.Body, threshold)

    def TestOCR(self, group: BaseGroup, threshold: float) -> float:
        return self.TestItem(group, self.OCR, threshold)

    def TestSubImages(self, image: OCRImage) -> bool:
        if len(self.Images) == 0: return False
        for imgPattern in self.Images:
            imgPath = os.path.join("images", imgPattern)
            imgNames = None
            if '*' in imgPattern:
                imgNames = glob(imgPath)
            else:
                imgNames = [imgPath]
            for imgName in imgNames:
                outPath = os.path.join(os.path.dirname(image.path), "result_" + os.path.basename(imgName))
                if checkForSubImage(image.path, imgName, outPath):
                    return True
        return False
    def TestFunctions(self, image: OCRImage) -> bool:
        if len(self.Functions) == 0: return False
        for funcName in self.Functions:
            if FUNCTIONS[funcName](image):
                return True
        return False
