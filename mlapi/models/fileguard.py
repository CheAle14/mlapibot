import logging
import os

class FileGuard:
    """
        Deletes the path after process has completed.
    """
    def __init__(self, path : str):
        self.path = path
    def __enter__(self):
        return self
    def __exit__(self, type, value, traceback):
        try:
            os.remove(self.path)
        except:
            logging.info("Attempted to delete " + self.path + " but there was an error.")

    