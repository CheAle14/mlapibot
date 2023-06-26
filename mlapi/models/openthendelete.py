from io import TextIOWrapper
import logging
import os

class OpenThenDelete:
    """
        Deletes the path after process has completed.
    """
    def __init__(self, path, *args, **kwargs):
        self.path = path
        self.args = args
        self.kwargs = kwargs
    def __enter__(self) -> TextIOWrapper:
        self.fd = open(self.path, *self.args, **self.kwargs)
        return self.fd
    def __exit__(self, type, value, traceback):
        try:
            self.fd.close()
            os.remove(self.path)
        except:
            logging.info("Attempted to close and delete " + self.path + " but there was an error.")

    