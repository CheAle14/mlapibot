import ocr
import unittest
import main
import os, sys
import re
import logging
from os import listdir
from os.path import isfile, join

def test_file(filename):
    text = ocr.getTextFromPath(
    os.path.join(path, filename), filename)
    text = text.lower()
    array =  re.findall(r"[\w']+", text)
    logging.info(array)
    scams = main.getScams(array)
    logging.info("{0}: {1}".format(filename, scams))
    return len(scams) > 0

def test_files_name(iFrom, iTo):
    failed = []
    for file in files:
        if not main.validImage(file):
            continue
        number = file[:file.find('.')]
        number = int(number)
        if number >= iFrom <= iTo:
            r = test_file(file)
            if not r:
                failed.append(file)
    return failed

class TestImages(unittest.TestCase):
    def test_batch(self):
        files = [f for f in listdir(path) if isfile(join(path,f))]
        for file in files:
            with self.subTest(file=file):
                if not main.validImage(file):
                    continue
                logging.debug("====== Start {0} ======".format(file))
                r = test_file(file)
                if "n" in file[:file.find('.')]:
                    self.assertFalse(r)
                else:
                    self.assertTrue(r)
                logging.debug("====== End {0} ======".format(file))


if __name__ == '__main__':
    main.load_scams()
    path = os.getcwd()
    path = os.path.join(path, "tests")
    os.chdir(path)
    logging.basicConfig(filename='test.log', level=logging.DEBUG)
    unittest.main()

