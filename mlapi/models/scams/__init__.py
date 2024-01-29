import os
from abc import ABC, abstractmethod
from typing import Dict, List, Tuple, Union

from PIL import Image
from praw.models import Comment, Submission
from strsimpy.weighted_levenshtein import WeightedLevenshtein
from typing_extensions import Any, Self

from mlapi.models.words import BaseGroup, BaseWord, OCRImage, RedditGroup


def ins_fn(c): return 1.0
def del_fn(c): return 1.0
def sub_fn(char_a, char_b):
    return 1.0

comparer = WeightedLevenshtein(substitution_cost_fn=sub_fn, insertion_cost_fn=ins_fn, deletion_cost_fn=del_fn)

class ScamContext:
    def __init__(self, data: 'MLAPIData', inner_thing: Union[Comment, Submission, None], title: str, body: str, images: List[OCRImage]):
        self.data = data
        self.inner = inner_thing
        self.title = RedditGroup(title)
        self.body = RedditGroup(body)
        self.images = images

    def groups(self):
        yield self.title
        yield self.body
        for x in self.images:
            yield x

    def push(self, name):
        for g in self.groups():
            g.push(name)
        
    def pop(self):
        for g in self.groups():
            g.pop()
    
    def keep_only(self, prefix, index):
        for g in self.groups():
            g.keep_only(prefix, index)

    def __enter__(self) -> Self:
        return self
    
    def __exit__(self, *args, **kwargs):
        for img in self.images:
            try:
                os.remove(img.path)
            except:
                pass
            try:
                os.remove(img.original_path)
            except:
                pass


class BaseScamChecker(ABC):
    def __init__(self, name: str, ignore_self_posts: bool, template: str, report: bool, blacklist: List[str]):
        self.name = name
        self.ignore_self_posts = ignore_self_posts
        self.template = template
        self.report = report
        self.blacklist = blacklist
        self.__dbg = False

    @staticmethod
    @abstractmethod
    def from_json(json: Dict[str, Any]) -> Self:
        raise NotImplementedError()
    
    def is_similar_word(self, wordA: str, wordB: str, distance = None):
        if distance is None:
            distance = comparer.distance(wordA, wordB)
        return distance <= 2 and (distance / max(len(wordA), len(wordB)) < 0.5)

    def parse_test_word(self, word: str):
        if word[0] == "!":
            new = word.lstrip("!")
            diff = len(word) - len(new)
            return (new, len(new) * (diff+1))
        return (word, len(word))

    def leven_distance(self, startAt: int, testStartAt: int, words: List[BaseWord], testing: List[str], _depth = 0) -> Tuple[float, float, float]:
        # start off by calculating the value of the items we skipped in the test string
        # as we may have started looking a couple words ahead, and we need to count
        # that against the overall distance of this test.
        overallDistance = sum([self.parse_test_word(x)[1] for x in testing[:testStartAt]])
        maximumPossibleDistance = overallDistance

        tentativeCon = []

        current = startAt
        testCurrent = testStartAt
        while current < len(words) and testCurrent < len(testing):
            testWord, testWordDistance = self.parse_test_word(testing[testCurrent])
            distance = comparer.distance(words[current].text, testWord)
            if self.is_similar_word(words[current].text, testWord, distance):
                words[current].seen_distance = distance
                tentativeCon.append(current)
                overallDistance += distance
                maximumPossibleDistance += max(len(words[current].text), testWordDistance)
                testCurrent += 1
            else:
                #print("    ", " " * _depth, self.name, "cannot find", testCurrent, testWord, "at", current, words[current])
                # we are missing a word.
                # three methods we consider:
                # 1) check to see whether two words have been squashed together, if possible.
                # 2) pretending the test word doesn't exist and attempting to find the next test word
                # 3) pretending the group word doesn't exist and attempting to find the same test word within the next few words
                
                # starting with (1):
                if (testCurrent + 1) < len(testing):
                    curWord = words[current].text
                    testOne, oneDist = self.parse_test_word(testing[testCurrent])
                    testTwo, twoDist = self.parse_test_word(testing[testCurrent + 1])
                    squashedTest = testOne + testTwo
                    distance = comparer.distance(curWord, squashedTest)
                    if self.is_similar_word(curWord, squashedTest, distance):
                        # we've found the words joined together, so we'll note the distance and move on
                        overallDistance += distance + 1 # for space missing
                        maximumPossibleDistance += max(len(curWord), oneDist + twoDist)
                        words[current].seen_distance = distance + 1
                        tentativeCon.append(current)
                        testCurrent += 2
                        current += 1
                        continue

                # then (2):
                best_next_test = (1000, None)
                for tryIndex in range(testCurrent+1, min(testCurrent+1 + 5, len(testing))):
                    tWord, _ = self.parse_test_word(testing[tryIndex])
                    distance = comparer.distance(words[current].text, tWord)
                    if self.is_similar_word(words[current].text, tWord, distance):
                        best_next_test = (distance, tryIndex)
                
                best_next_group = (1001, None)
                for tryIndex in range(current+1, min(current+1 + 5, len(words))):
                    distance = comparer.distance(words[tryIndex].text, testWord)
                    if self.is_similar_word(words[tryIndex].text, testWord, distance):
                        best_next_group = (distance, tryIndex)
                
                if best_next_test[1] is None and best_next_group[1] is None:
                    # hidden achievement (4): unable to find either next test or next group.
                    # so our consecutive searching ends here.
                    # calculate all the remaining missing distance
                    remaining_test_sum = sum([self.parse_test_word(x)[1] for x in testing[testCurrent:]])
                    remaining_group_sum = 0 # sum([len(x.text) for x in words[current:]])
                    distance = max(remaining_test_sum, remaining_group_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    #print("    ", " " * _depth, self.name, "unable to relock", remaining_test_sum, remaining_group_sum)
                    break
                elif best_next_group[1] is None or best_next_test[0] < best_next_group[0]:
                    # scenario (2): we've found the test word later on, maybe.
                    skipped_test_sum = sum([self.parse_test_word(x)[1] for x in testing[testCurrent:best_next_test[1]]])
                    distance = max(distance, skipped_test_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    testCurrent = best_next_test[1] + 1
                    current += 1

                    #print("    ", " " * _depth, self.name, "found text word again", best_next_test)
                    continue
                else:
                    # scenario (3):
                    skipped_group_sum = sum([len(x.text) for x in words[current:best_next_group[1]]])
                    distance = max(distance, skipped_group_sum)
                    overallDistance += distance
                    maximumPossibleDistance += distance
                    current = best_next_group[1] + 1
                    testCurrent += 1
                    #print("    ", " " * _depth, self.name, "found next current", best_next_group)
                    continue


            current += 1

        if maximumPossibleDistance == 0:
            #print("    ", " "*_depth, self.name, "found nothing")
            return (0, 0, 0)
        #print("    ", " "*_depth, self.name, "tentative:", tentativeCon)
        if len(tentativeCon) > max(2, len(testing) * 0.1):
            for idx in tentativeCon:
                words[idx].consecutive = True
        #print("    ", " "*_depth, self.name, (startAt, testStartAt),"->", (current, testCurrent), "got", overallDistance, "of", maximumPossibleDistance)
        return (1 - (overallDistance / maximumPossibleDistance), overallDistance, maximumPossibleDistance)
            
    def best_leven_distance(self, group: BaseGroup, words: List[BaseWord], testing: List[str]) -> float:
        if self.__dbg:
            print("BLD:", testing)
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
                elif self.__dbg:
                    print(words[i], "not", testing[start])


        if self.__dbg:
            print("Start:", starts)
        best = None
        bestIdx = None
        prefix = self.name + "-leven-"
        for i in range(len(starts)):
            pair = starts[i]
            group.push(prefix + str(i))
            score, _, _ = self.leven_distance(pair[0], pair[1], words, testing)
            #print(self.name, pair, "normalised:", score)
            if best is None or score > best:
                best = score
                bestIdx = i
        group.keep_only(prefix, bestIdx)
        return best or 0

    def try_match(self, needles: List[List[str]], haystack: BaseGroup, threshold: float) -> float:
        if len(needles) == 0: return 0

        highest = 0
        highestIndex = None
        highestArray = None
        #group.dump()
        words = [word for word in haystack.words if word.conf >= 70]
        #print("Seen:", [f"{i}:\"{words[i]}\"" for i in range(len(words))])
        prefix = self.name + "-scam-"
        for i, testArray in enumerate(needles):
            if self.__dbg:
                print("=======BREAK:  ")
                print("Needle:", testArray)
            haystack.push(prefix + str(i))
            perc = self.best_leven_distance(haystack, words, testArray)
            #print(self.name, "best =", perc)

            if perc > highest:
                highest = perc
                highestArray = testArray
                if perc > threshold:
                    highestIndex = i
        if self.__dbg:
            print(str(int(highest * 100)).zfill(2) + "%", self.name, " ".join(testArray))
        haystack.keep_only(prefix, highestIndex)
        return highest


    @abstractmethod
    def matches(self, context: ScamContext, THRESHOLD: float) -> float:
        raise NotImplementedError()
    
    def is_blacklisted(self, context: ScamContext, THRESHOLD: float) -> bool:
        if len(self.blacklist) == 0: return False
        needles = [text.split() for text in self.blacklist]
        if context.title:
            return self.try_match(needles, context.title, THRESHOLD) >= THRESHOLD
        if context.body:
            return self.try_match(needles, context.body, THRESHOLD) >= THRESHOLD
        
        return False