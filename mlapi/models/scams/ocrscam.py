from typing import Dict, List

from typing_extensions import Any, Self

from mlapi.models.scams import BaseScamChecker, ScamContext


class OCRScamChecker(BaseScamChecker):
    def __init__(self, name: str, ignore_self_posts: bool, template: str, report: bool, blacklist: List[str], phrases: List[str]):
        super().__init__(name, ignore_self_posts, template, report, blacklist)
        self.phrases = [text.split() for text in phrases]


    @staticmethod
    def from_json(json: Dict[str, Any]) -> Self:
        return OCRScamChecker(
            json.get("name"),
            json.get("ignore_self_posts", False),
            json.get("template", "default"),
            json.get("report", False),
            json.get("blacklist", []),
            json.get("ocr")
        )

    def matches(self, context: ScamContext, THRESHOLD: float = 0.8):
        scores = [self.try_match(self.phrases, image, THRESHOLD) for image in context.images]
        return any(score >= THRESHOLD for score in scores)
