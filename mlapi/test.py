from typing import List
import unittest
import os, sys
import re
import logging
from os import listdir
from os.path import isfile, join

from mlapi.main import MLAPIData
from mlapi.ocr import getTextFromPath

NO_SCAMS_KEYWORD = "none"



def do_test(image: str, expecting: List[str], data: MLAPIData):

    response = data.getScamsForImage(getTextFromPath(image), data.SCAMS)
    
    error = False
    seen = []
    for scam, conf in response.Scams.items():
        if scam.Name in expecting:
            seen.append(scam.Name)
        else:
            error = True
            print("[FAILED]", image, "had unexpected", scam.Name, "at", conf)
    
    unseen = [name for name in expecting if name not in seen]
    if unseen:
        error = True
        print("[FAILED]", image, "was missing", unseen, "it has instead:", [key.Name for key, v in response.Scams.items()])
        for ocr in response.OCRGroups:
            #ocr.dump()
            ocr.getSeenCopy().show()
            ocr.getScamCopy().show()
    if not error:
        print(image, "okay.")
    return error

    

def do_tests(folder, data: MLAPIData, names = None):
    error = False
    for name in os.listdir(folder):
        path = os.path.join(folder, name)
        child_names = [x for x in os.path.basename(name).split('_') if x != NO_SCAMS_KEYWORD] if names == None else names
        if os.path.isfile(path):
            error = do_test(path, child_names, data) or error
        else:
            error = do_tests(path, data, child_names) or error
    return error


def run_all_tests(dir = None):
    datadir = os.path.join(dir or os.getcwd(), "mlapi", "data")
    data = MLAPIData(datadir)
    testdir = os.path.join(dir or os.getcwd(), "tests")
    try:
        error = do_tests(testdir, data)
    except Exception as e:
        print(e)
        exit(2)
    if error:
        print("Tests failed.")
        exit(1)
    else:
        print("Tests passed")
        exit(0)

if __name__ == '__main__':
    run_all_tests()