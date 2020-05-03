import logging
try:
    from PIL import Image
except ImportError:
    import Image
import pytesseract
import cv2
import os, tempfile
if os.name == 'nt':
    pytesseract.pytesseract.tesseract_cmd = r'C:\Program Files (x86)\Tesseract-OCR\tesseract'


def getTextFromPath(path, filename):
    image = cv2.imread(path)
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    gray = cv2.bitwise_not(gray)
    filename = "corrected_{}.png".format(filename)
    correctedPath = os.path.join(tempfile.gettempdir(), filename)
    cv2.imwrite(correctedPath, gray)
    return pytesseract.image_to_string(Image.open(correctedPath))

