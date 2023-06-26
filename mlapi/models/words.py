from typing import List
from PIL import Image, ImageDraw
import re
import pytesseract


class BaseWord:
    def __init__(self):
        self._present = False
        self._consecutive = False
        self._o = {}
        self.reset()

    def reset(self):
        self.consecutive = self._consecutive
        self.present = self._present
    def lockin(self):
        self._consecutive = self._consecutive or self.consecutive
        self._present = self._present or self.present

    def __str__(self):
        lr = ''
        if self.present:
            lr = '_'
        if self.consecutive:
            lr += '**'
        if len(lr) == 0: return self.text
        return lr + self.text + lr[::-1]
    
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
    def reset(self):
        for word in self.words:
            word.reset()
    def lockin(self):
        for word in self.words:
            word.lockin()

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

        print(self.left, self.top, self.width, self.height, self.text.encode(), len(self.text))

        self.drawBoundaryBox(draw, outline, "white")
        textboundbox = draw.textbbox((self.left, self.top), self.text, align="center")
        textwidth = textboundbox[2] - textboundbox[0]
        textheight = textboundbox[3] - textboundbox[1]
        
        textL = (self.width - textwidth) / 2
        textT = (self.height - textheight) / 2
        draw.text((self.left + textL, self.top + textT), self.text, fill=outline)
    
    def drawBoundaryBox(self, draw: ImageDraw, outline, fill = None, padding=0):
        xy = [(self.left-padding, self.top-padding), (self.left + self.width+padding, self.top + self.height+padding)]
        draw.rectangle(xy, fill=fill, outline=outline)




class OCRImage(BaseGroup):
    def __init__(self, path):
        super().__init__()
        self.path = path
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
            if word.consecutive:
                word.drawBoundaryBox(draw, "red", padding=2)
            elif word.present:
                word.drawBoundaryBox(draw, "blue", padding=2)
        return copy