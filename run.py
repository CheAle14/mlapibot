import os
import sys
if len(sys.argv) == 2 and sys.argv[1]:
    import mlapi.ocr
    path = sys.argv[1]
    if os.path.exists(path):
        print(mlapi.ocr.getTextFromPath(path, os.path.basename(path)))
        exit(0)
    sys.stderr.write("Path does not exist")
    exit(1)
else:
    os.chdir(os.path.join(os.getcwd(), "mlapi"))
    import mlapi.main
    print("Running...")
    mlapi.main.start()
