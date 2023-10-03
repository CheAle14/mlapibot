import os
import sys
from mlapi.main import MLAPIReddit, MLAPIData
from mlapi.ocr import getTextFromPath

dir = os.path.join(os.getcwd(), "mlapi", "data")
if len(sys.argv) == 2:
    path = sys.argv[1]
    if path == "test":
        import mlapi.test
        mlapi.test.run_all_tests()
    else:
        data = MLAPIData(dir)
        if path.startswith("http"):
            image = data.handleUrl(path)
        else:
            try:
                image = getTextFromPath(path)
            except Exception as e:
                print("Error:", e)
                exit(1)
        with image:
            builder = data.getScamsForImage(image, data.SCAMS)
            print(builder.getScamText())
            for ocr in builder.OCRGroups:
                #ocr.dump()
                ocr.getSeenCopy().show()
                ocr.getScamCopy().show()
        exit(0)
else:
    print("Running...")
    client = MLAPIReddit(dir)
    client.run_forever()
    
