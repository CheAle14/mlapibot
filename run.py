import os
import sys

os.chdir(os.path.join(os.getcwd(), "mlapi"))
if len(sys.argv) == 2:
    path = sys.argv[1]
    import mlapi.main
    if path.startswith("http"):
        image = mlapi.main.handleUrl(path)
        image.getSeenCopy().show()
    else:
        from mlapi.models.words import OCRImage
        mlapi.main.load_scams()
        try:
            import mlapi.ocr
            image = mlapi.ocr.getTextFromPath(path)
        except Exception as e:
            print("Error:", e)
            exit(1)
        builder = mlapi.main.getScamsForImage(image, mlapi.main.SCAMS)
        print(builder.getScamText())
        ocr: OCRImage = None
        for ocr in builder.OCRGroups:
            #ocr.dump()
            ocr.getSeenCopy().show()
            ocr.getScamCopy().show()
    exit(0)
else:
    import mlapi.main
    print("Running...")
    mlapi.main.start()
