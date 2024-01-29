import os
from typing import Dict, List

import cv2 as cv
import numpy as np
from typing_extensions import Any, Self

from mlapi.models.scams import BaseScamChecker, ScamContext


class ImgScamChecker(BaseScamChecker):
    def __init__(self, name: str, ignore_self_posts: bool, template: str, report: bool, blacklist: List[str], img_name: str):
        super().__init__(name, ignore_self_posts, template, report, blacklist)
        if type(img_name) == str:
            img_name = [img_name]
        self.img_names: List[str] = img_name

    @staticmethod
    def from_json(json: Dict[str, Any]) -> Self:
        return ImgScamChecker(
            json.get("name"),
            json.get("ignore_self_posts", False),
            json.get("template", "default"),
            json.get("report", False),
            json.get("blacklist", []),
            json.get("img")
        )
    
    def _try_template(self, context: ScamContext, name) -> bool:
        path = os.path.join(context.data.data_dir, "images", name)
        template = cv.imread(path, cv.IMREAD_COLOR)
        assert template is not None, f"template image could not be read at {path}"
        w, h, cd = template.shape

        for ocr_img in context.images:
            img = cv.imread(ocr_img.original_path, cv.IMREAD_COLOR).copy()
            assert img is not None, "file could not be read, check with os.path.exists()"
            if img.shape[0] < w: continue
            if img.shape[1] < h: continue
            # Apply template Matching
            res = cv.matchTemplate(img, template, cv.TM_CCOEFF_NORMED)
            # Store the coordinates of matched area in a numpy array 
            loc = np.where(res >= 0.95) 
            
            # Draw a rectangle around the matched region. 
            for pt in zip(*loc[::-1]): 
                ocr_img.rectangles.append((pt, (pt[0] + w, pt[1] + h)))
                return True
        return False

    def matches(self, context: ScamContext, THRESHOLD: float) -> float:
        return 1 if any(self._try_template(context, t) for t in self.img_names) else 0