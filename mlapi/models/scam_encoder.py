import os, re
from typing import List
from json import JSONEncoder

class ScamEncoder(JSONEncoder):
        def default(self, o):
            return o.__dict__