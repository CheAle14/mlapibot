from typing import List
from PIL import Image, ImageDraw
import re
import pytesseract
import os


class BaseWord:
    def __init__(self):
        self._seen_distance = 2**16
        self._consecutive = False
        self._stack = [(2**16, False, "Start")]
        self._o = {}

    def push(self, name = None):
        self._stack.append((self.seen_distance, self.consecutive, name))
    def pop(self):
        if len(self._stack) > 1:
            return self._stack.pop()
        return None
    def keep_only(self, prefix, index):
        remove = []
        index = str(index)
        selected = None
        for item in self._stack:
            if item[2].startswith(prefix):
                remove.append(item)
                if index is not None and item[2].endswith("-" + index):
                    selected = item
        for r in remove:
            self._stack.remove(r)
        if selected:
            self.seen_distance = min(selected[0], self.seen_distance)
            self.consecutive = self.consecutive or selected[1]

    def __str__(self):
        lr = ''
        if self.seen_distance < 3:
            lr = '_'
        if self.consecutive:
            lr += '**'
        if len(lr) == 0: return self.text
        return lr + self.text + lr[::-1]
    
    def __repr__(self): return f"{self.text} {self.conf}"
    
    @property
    def seen_distance(self):
        return self._stack[-1][0]
    
    @seen_distance.setter
    def seen_distance(self, value):
        self._stack[-1] = (value, self.consecutive, self.name)
    
    @property
    def consecutive(self):
        return self._stack[-1][1]
    
    @consecutive.setter
    def consecutive(self, value):
        self._stack[-1] = (self.seen_distance, value, self.name)

    @property
    def name(self):
        return self._stack[-1][2]

    
    @property
    def text(self):
        return self._o.get('text', "")
    @text.setter
    def text(self, value: str):
        self._o['text'] = re.sub("[^\w']", "", str(value).lower())
    
    @property
    def line(self):
        return self._o.get('line_num', 0)
    @line.setter
    def line(self, value):
        if not isinstance(value, int):
            value = int(str(value), 10)
        self._o['line_num'] = value

    @property
    def conf(self):
        return self._o.get("conf", 100)
    @conf.setter
    def conf(self, value):
        self._o["conf"] = int(value, 10)
    

class BaseGroup:
    def __init__(self):
        self.words : List[BaseWord] = []
    def __str__(self):
        word : BaseWord = None
        lastLine = 0
        s = " "
        for word in self.words:
            s += str(word) + '\n' if lastLine != word.line else ' '
        return s
    def push(self, name = None):
        #print("++", name or "None")
        for word in self.words:
            word.push(name)
    def pop(self):
        #print("--")
        for word in self.words:
            word.pop()
    def keep_only(self, prefix, index):
        #print("==", prefix, "None" if index is None else index)
        for word in self.words:
            word.keep_only(prefix, index)
    def dump(self):
        print("line", "stack", "conf", "text", sep='\t')
        for word in self.words:
            print(word.line, word._stack, word.conf, word.text.encode(), sep='\t')

class RedditWord(BaseWord):
    def __init__(self, word, line):
        super().__init__()
        self.text = word
        self.line = line

class RedditGroup(BaseGroup):
    def __init__(self, text: str):
        super().__init__()
        lineNo = 0
        for line in text.splitlines():
            for word in line.split(' '):
                self.words.append(RedditWord(word, lineNo))
            lineNo += 1
         

class OCRImageWord(BaseWord):
    def __init__(self, image, line):
        super().__init__()
        values = line.split('\t')
        for i in range(min(len(image.keys), len(values))):
            value = values[i]
            key = image.keys[i]
            if key == "text":
                self.text = value
            else:
                try:
                    value = int(value, 10)
                except:
                    pass
                self._o[key] = value
    
    @property
    def left(self):
        return self._o["left"]
    @property
    def top(self):
        return self._o["top"]
    @property
    def width(self):
        return self._o["width"]
    @property
    def height(self):
        return self._o["height"]
    
    
    def drawSeenTextBox(self, draw: ImageDraw):
        if self.conf <= 5 or not self.text:
            return
        if self.conf < 25:
            outline = "red"
        elif self.conf < 50:
            outline = "orange"
        elif self.conf < 80:
            outline = "blue"
        else:
            outline = "green"

        #print(self.left, self.top, self.width, self.height, self.text.encode(), len(self.text))

        self.drawBoundaryBox(draw, outline, "white")
        textboundbox = draw.textbbox((self.left, self.top), self.text, align="center")
        textwidth = textboundbox[2] - textboundbox[0]
        textheight = textboundbox[3] - textboundbox[1]
        
        textL = (self.width - textwidth) / 2
        textT = (self.height - textheight) / 2
        draw.text((self.left + textL, self.top + textT), self.text, fill=outline)

    def drawScamBox(self, draw: ImageDraw):
        if len(self.text) == 0: return
        if self.seen_distance > len(self.text): return
        normalised = 1-(self.seen_distance / len(self.text))
        if normalised < 0.25:
            outline = "red"
        elif normalised <= 0.50:
            outline = "orange"
        elif normalised < 0.80:
            outline = "blue"
        else:
            outline = "green"

        if self.consecutive:
            self.drawBoundaryBox(draw, outline, padding=2, width=2)
        else:
            self.drawBoundaryBox(draw, outline, padding=2)
    
    def drawBoundaryBox(self, draw: ImageDraw, outline, fill = None, padding=0, width=1):
        xy = [(self.left-padding, self.top-padding), (self.left + self.width+padding, self.top + self.height+padding)]
        draw.rectangle(xy, fill=fill, outline=outline, width=width)




class OCRImage(BaseGroup):
    def __init__(self, path, original_path = None):
        super().__init__()
        self.path = path
        self.original_path = original_path
        self.image = Image.open(path)
        self.rawlines = str(pytesseract.image_to_data(self.image)).strip().splitlines()
        self.keys = self.rawlines[0].split('\t')
        self.words = []
        for i in range(1, len(self.rawlines)):
            word = OCRImageWord(self, self.rawlines[i])
            self.words.append(word)
    def copy(self):
        return self.image.copy()
    def getSeenCopy(self):
        copy = self.copy().convert("RGB")
        draw = ImageDraw.Draw(copy)
        for word in self.words:
            word.drawSeenTextBox(draw)
        return copy
    def getScamCopy(self):
        copy = self.copy().convert("RGB")
        draw = ImageDraw.Draw(copy)
        word: OCRImageWord = None
        for word in self.words:
            word.drawScamBox(draw)
        return copy
    def __enter__(self):
        print(">", self.path)
        print(">", self.original_path)
    def __exit__(self, type, value, tb):
        try:
            print("<", self.path)
            os.remove(self.path)
        except FileNotFoundError:
            pass
        try:
            print("<", self.original_path)
            os.remove(self.original_path)
        except FileNotFoundError:
            pass