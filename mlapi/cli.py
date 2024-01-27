import os
import sys
from mlapi.main.data import MLAPIData

from mlapi.main.reddit import MLAPIReddit
from mlapi.ocr import getTextFromPath

def watch():
    dir = sys.argv[1] if len(sys.argv) == 2 else os.path.join(os.path.dirname(__file__), 'data')
    client = MLAPIReddit(dir)
    client.run_forever()

def check():
    if len(sys.argv) not in [2, 3]:
        raise ValueError("Usage: mlapibot-check [url or path]")
    path = sys.argv[1]
    dir = sys.argv[2] if len(sys.argv) == 3 else os.path.join(os.path.dirname(__file__), 'data')
    data = MLAPIData(dir)
    if path.startswith("http"):
        image = data.handleUrl(path)
    else:
        try:
            image = getTextFromPath(path)
        except Exception as e:
            print("Error:", e)
            return 1
    with image:
        builder = data.getScamsForImage(image, data.SCAMS)
        print(builder.getScamText())
        for ocr in builder.OCRGroups:
            #ocr.dump()
            ocr.getSeenCopy().show()
            ocr.getScamCopy().show()
        return 0

def test() -> int:
    import mlapi.test
    return mlapi.test.run_all_tests()

if __name__ == "__main__":
    watch()