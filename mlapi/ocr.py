import logging

from mlapi.models.fileguard import FileGuard
try:
    from PIL import Image
except ImportError:
    import Image
import pytesseract
import cv2
import os, tempfile, numpy
from colorsys import rgb_to_hls, hls_to_rgb

def processImage(image):
    avg_color_per_row = numpy.average(image, axis=0)
    avg_color = numpy.average(avg_color_per_row, axis=0) # Blue Green Red.
    hls = rgb_to_hls(avg_color[2], avg_color[1], avg_color[0])
    logging.info("Lightness of image: " + str(hls[1])) # between 255 (light) and 0 (dark)
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)


    # Tesseract works best with dark text on a light background.
    # So, if the image is (on average) dark we can try and invert it.
    if hls[1] < 80:
        logging.info("Inverting image.")
        gray = cv2.bitwise_not(gray)    

    return gray

def getTextFromPath(path, filename):
    image = cv2.imread(path, cv2.IMREAD_COLOR)

    processed = processImage(image)

    filename = "corrected_{}".format(filename)
    if '.' not in filename:
        filename = filename + ".png"
    correctedPath = os.path.join(tempfile.gettempdir(), filename)
    logging.info("Corrected -> " + correctedPath)
    with FileGuard(correctedPath):
        cv2.imwrite(correctedPath, processed)
        return pytesseract.image_to_string(Image.open(correctedPath))




