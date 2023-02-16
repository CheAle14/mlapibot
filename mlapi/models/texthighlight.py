class TextNode:
    def __init__(self, index : int, text : str, pairIdx : int = None, pairID = None):
        self.idx = index
        self.txt = text
        self.pairIdx = pairIdx
        self.pairID = pairID
    def __repr__(self) -> str:
        return f"'{self.txt}' @ {self.idx} ({self.pairIdx})"
TAGS = [
    "<>",
    "()",
    "{}",
    "[]"
]
class TextHighlight:
    def __init__(self, rawtext : str):
        self.rawtext = rawtext
        self.italic_words = []
        self.nodes = []
        self.wordOffset = 0
        self.pairs = []
    #def appendNode(self, node : TextNode):
        #found = False
        #for i in range(len(self.nodes)):
        #    other = self.nodes[i]
        #    if other.idx > node.idx and other.pairIdx < node.pairIdx:
        #
        #        self.nodes.insert(i, node)
        #        found=True
        #        break
        #if not found:
    def append(self, index : int, text : str, pairIdx : int = None, pairID : int = None):
        self.nodes.append(TextNode(index, text, pairIdx, pairID))
    def setItalic(self, wordIndex : int):
        val = wordIndex + self.wordOffset
        if val not in self.italic_words:
            self.italic_words.append(val)

    def getIndexes(self, wordIndex : int, length : int = 1):
        spaces = 0 - self.wordOffset
        startIdx = None
        endIdx = None
        foundCount = 0
        for i in range(0, len(self.rawtext)):
            if spaces == wordIndex and startIdx is None:
                startIdx = i
            if self.rawtext[i] == ' ':
                spaces += 1
                if startIdx is not None:
                    foundCount += 1
            if foundCount == length:
                endIdx = i
                break
        if endIdx is None:
            endIdx = len(self.rawtext)
        return (startIdx, endIdx)

    def addPair(self, startIdx, endIdx, startTag, endTag):
        key = (startIdx, endIdx)
        for existing in self.pairs:
            if existing[0] == key[0] and existing[1] == key[1]:
                return # pair already exists, don't bother
        self.pairs.append((startIdx, endIdx))
        self.append(startIdx, startTag, pairIdx=endIdx, pairID = len(self.pairs))
        self.append(endIdx, endTag, pairIdx=startIdx, pairID = len(self.pairs))

    def wrapword(self, wordIndex : int, length : int, startTag : str, endTag : str):
        (startIdx, endIdx) = self.getIndexes(wordIndex, length)
        self.addPair(startIdx, endIdx, startTag, endTag)
    def autowrap(self, wordIndex : int, length : int):
        (startIdx, endIdx) = self.getIndexes(wordIndex, length)
        ignore_tags = []
        for node in self.nodes:
            if node.idx >= startIdx and node.idx <= endIdx:
                for tag_type in TAGS:
                    if node.txt in tag_type and tag_type not in ignore_tags:
                        ignore_tags.append(tag_type)
                        break
        for tag_type in TAGS:
            if tag_type in ignore_tags: continue
            self.addPair(startIdx, endIdx, tag_type[0], tag_type[1])
            return
        ## not enough tags
        self.addPair(startIdx, endIdx, f":{wordIndex}:", f";{wordIndex};")


    
    def build(self):
        self.nodes.sort(key=lambda item: (item.idx, -item.pairIdx))
        originalIndex = 0
        string = []
        wordCount = 0
        is_italic = False
        while originalIndex < (len(self.rawtext) + 1):
            index_before_nodes = len(string)
            for node in self.nodes:
                #if node.idx > originalIndex: break
                if node.idx == originalIndex:
                    string.append(node.txt)
            if originalIndex < len(self.rawtext):
                char = self.rawtext[originalIndex]
                if not is_italic and wordCount in self.italic_words:
                    is_italic = True
                    string.append("*")
                if char == ' ':
                    if is_italic:
                        is_italic = False
                        string.insert(index_before_nodes, "*") # put astersik before brackets
                    wordCount += 1
                string.append(char)
            originalIndex += 1
        return "".join(string)
    def tofile(self):
        self.nodes.sort(key=lambda item: (abs(item.idx - item.pairIdx)))
        lines = []
        lines.append(self.rawtext)
        done = []
        for node in self.nodes:
            if node.pairID in done: continue
            done.append(node.pairID)
            start = node.idx
            end = node.pairIdx
            if start > end:
                start, end = end, start
            line = ""
            for i in range(len(self.rawtext)):
                if i < start or i > end:
                    line += ' '
                elif i == start or i == end:
                    line += "^"
                else:
                    line += "-"
            lines.append(line)
        return "\n".join(lines)
        
                

if __name__ == "__main__":
    tester = TextHighlight("The quick brown fox jumps over the lazy dog")
    tester.autowrap(0, 1)
    tester.autowrap(0, 1)
    tester.autowrap(1, 1)
    tester.autowrap(1, 1)
    print(tester.build())
    with open("tester.txt", "w") as f:
        f.write(tester.tofile())