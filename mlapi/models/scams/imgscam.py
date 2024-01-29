import glob
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
    


    def _sift_feature_match(self, context, name, img1, img2):
        MIN_MATCH_COUNT = 10
        # Initiate SIFT detector
        sift = cv.SIFT_create(contrastThreshold=0.03, edgeThreshold=20)
        # find the keypoints and descriptors with SIFT
        kp1, des1 = sift.detectAndCompute(img1,None)
        kp2, des2 = sift.detectAndCompute(img2,None)
        FLANN_INDEX_KDTREE = 1
        index_params = dict(algorithm = FLANN_INDEX_KDTREE, trees = 5)
        search_params = dict(checks = 50)
        flann = cv.FlannBasedMatcher(index_params, search_params)
        matches = flann.knnMatch(des1,des2,k=2)
        # store all the good matches as per Lowe's ratio test.
        good = []
        for m,n in matches:
            if m.distance < 0.7*n.distance:
                good.append(m)
        if len(good)>=MIN_MATCH_COUNT:
            src_pts = np.float32([ kp1[m.queryIdx].pt for m in good ]).reshape(-1,1,2)
            dst_pts = np.float32([ kp2[m.trainIdx].pt for m in good ]).reshape(-1,1,2)
            M, mask = cv.findHomography(src_pts, dst_pts, cv.RANSAC,5.0)
            matchesMask = mask.ravel().tolist()
            h,w = img1.shape
            pts = np.float32([ [0,0],[0,h-1],[w-1,h-1],[w-1,0] ]).reshape(-1,1,2)
            dst = cv.perspectiveTransform(pts,M)
            img2 = cv.polylines(img2,[np.int32(dst)],True,255,3, cv.LINE_AA)
        else:
            print( "Not enough matches are found - {}/{}".format(len(good), MIN_MATCH_COUNT) )
            matchesMask = None
            dst = None

        # draw_params = dict(matchColor = (0,255,0), # draw matches in green color
        #            singlePointColor = None,
        #            matchesMask = matchesMask, # draw only inliers
        #            flags = 2)
        #img3 = cv.drawMatches(img1,kp1,img2,kp2,good,None,**draw_params)
        #cv.imwrite(f"output_{name}", img3)
        return np.int32(dst) if dst is not None else None


    def _try_template(self, context: ScamContext, path) -> bool:
        name = os.path.basename(path)
        if not os.path.exists(path):
            raise ValueError(f"No image template at {path}")
        try:
            img1 = cv.imread(path, cv.IMREAD_GRAYSCALE)
        except Exception:
            print(f"Failed to read image template {path}")
            raise
        assert img1 is not None, f"template image could not be read at {path}"

        print(">", name)
        for ocr_img in context.images:
            img2 = cv.imread(ocr_img.original_path, cv.IMREAD_GRAYSCALE).copy()
            assert img2 is not None, "file could not be read"
            rect = self._sift_feature_match(context, name, img1, img2)
            if rect is not None:
                rect = [x[0] for x in rect.tolist()]
                xs = [xy[0] for xy in rect]
                ys = [xy[1] for xy in rect]
                rect = [min(xs), min(ys), max(xs), max(ys)]
                print(rect)
                ocr_img.rectangles.append(rect)
                return True
        return False

    def _get_paths(self, context: ScamContext) -> List[str]:
        paths = []
        for name in self.img_names:
            path = os.path.join(context.data.data_dir, "images", name)
            if '*' in name:
                for path in glob.glob(path):
                    if path not in paths:
                        paths.append(path)
            else:
                if path not in paths:
                    paths.append(path)
        return paths

        
    def matches(self, context: ScamContext, THRESHOLD: float) -> float:
        return 1 if any(self._try_template(context, t) for t in self._get_paths(context)) else 0