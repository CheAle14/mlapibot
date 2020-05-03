from typing import List
class Scam:
    def __init__(self, name: str, reason: str, texts: List[str]):
        self.Name = name
        self.Reason = reason
        self.Texts = texts

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
        for testString in self.Texts:
            testArray = testString.split(' ')
            contain = self.numWordsContain(words, testArray)
            inOrder = self.phrasesInOrder(words, testArray)
            total = contain + inOrder
            perc = total / (len(testArray) * 2)
            if perc > highest:
                highest = perc
        return highest

