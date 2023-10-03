import logging

from mlapi.models.words import OCRImage

try:
    from PIL import Image, ImageDraw
except ImportError:
    import Image
import pytesseract
import cv2
import os
import tempfile
import numpy
import re
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

    
def getTextFromPath(path: str, filename: str = None) -> OCRImage:
    if filename is None:
        filename = os.path.basename(path)

    image = cv2.imread(path, cv2.IMREAD_COLOR)



    processed = processImage(image)

    filename = "corrected_{}".format(filename)
    if '.' not in filename:
        filename = filename + ".png"
    correctedPath = os.path.join(tempfile.gettempdir(), filename)
    logging.info("Corrected -> " + correctedPath)
    cv2.imwrite(correctedPath, processed)
    return OCRImage(correctedPath, path)
    
def checkForSubImage(testingPath, templatePath, outputPath = None):
    img_rgb = cv2.imread(testingPath)
    template = cv2.imread(templatePath)
    w, h = template.shape[:-1]

    res = cv2.matchTemplate(img_rgb, template, cv2.TM_CCOEFF_NORMED)
    threshold = 0.95
    loc = numpy.where(res >= threshold)
    hasMatch = False
    for pt in zip(*loc[::-1]):  # Switch columns and rows
        hasMatch = True
        if outputPath:
            cv2.rectangle(img_rgb, pt, (pt[0] + w, pt[1] + h), (0, 0, 255), 2)
    if outputPath:
        cv2.imwrite(outputPath, img_rgb)
    return hasMatch


def roundPixel(pixel):
    opt = []
    for v in list(pixel):
        rem = v % 10
        if rem < 5:
            opt.append(v - rem)
        else:
            opt.append(v + (10 - rem))
        if len(opt) == 3: break # discard alpha channel
    return tuple(opt)
    

def _getDistinctColors(img : Image.Image, startX, startY, distance):
    clrs = []
    for offset in range(distance):
        coord = (startX + offset, startY + offset)
        if coord[0] >= img.size[0] or coord[1] >= img.size[1]:
            break
        pixel = img.getpixel(coord)
        pixel = roundPixel(pixel)
        if pixel not in clrs:
            clrs.append(pixel)
    return clrs

def _drawDiagonal(img : Image.Image, startX, startY, distance, clr):
    for offset in range(distance):
        coord = (startX + offset, startY + offset)
        if coord[0] >= img.size[0] or coord[1] >= img.size[1]:
            break
        img.putpixel(coord, clr)


DISCORD_LOGO_APPROX = [[(30, 30, 40), (30, 30, 30)], # dark bg
                       [(90, 100, 240)], # blurple
                       [(240, 70, 70), (240, 60, 70)] # red notif
                      ]
def _checkForLogoColors(img : Image.Image, index):
    if index < 0:
        x = -index
        y = 0
    else:
        y = index
        x = 0
    dist = min(300, img.size[0])
    colors = _getDistinctColors(img, x, y, dist)

    seenIndex = 0
    for clr in colors:
        if clr in DISCORD_LOGO_APPROX[seenIndex]:
            seenIndex += 1
            if seenIndex == len(DISCORD_LOGO_APPROX):
                break
    to_draw = [(0, 0, 0), (0, 0, 255), (0, 255, 0), (255, 0, 0)]
    _drawDiagonal(img, x, y, dist, to_draw[seenIndex])
    if seenIndex == len(DISCORD_LOGO_APPROX):
        #_drawDiagonal(img, x, y, dist, (255, 0, 0))
        return True
    return False



def checkForDiscordLogo(image: OCRImage):
    # Idea is to scan a diagonal line in the top left of the image
    # to try and find, in order, pixels of:
    # (1) a dark background
    # (2) the blurple color of the logo
    # (3) the red color of the (1) notification.
    
    img = Image.open(image.original_path).convert("RGB")
    f = False
    for tester in range(-100, 250, 5):
        if _checkForLogoColors(img, tester):
            print("Found colors at diagonal", tester)
            f = True
    #img.save("result.png", "PNG")
    return f

FUNCTIONS = {
    "home_ds_logo": checkForDiscordLogo
}