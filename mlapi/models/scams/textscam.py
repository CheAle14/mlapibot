from typing import Dict, List

from typing_extensions import Any, Self

from mlapi.models.scams import BaseScamChecker, ScamContext


class TextScamChecker(BaseScamChecker):
    def __init__(self, name: str, ignore_self_posts: bool, template: str, report: bool, blacklist: List[str], title: List[str], body: List[str]):
        super().__init__(name, ignore_self_posts, template, report, blacklist)
        if not title and not body:
            raise ValueError("One of title or body must be populated")
        self.title = [line.split() for line in title]
        self.body = [line.split() for line in body]

    @staticmethod
    def from_json(json: Dict[str, Any]) -> Self:
        return TextScamChecker(
            json.get("name"),
            json.get("ignore_self_posts", False),
            json.get("template", "default"),
            json.get("report", False),
            json.get("blacklist", []),
            json.get("title", []),
            json.get("body", [])
        )

    def matches(self, context: ScamContext, THRESHOLD: float = 0.8):
        scores = [
            self.try_match(self.title, context.title, THRESHOLD) if context.title else 0,
            self.try_match(self.body, context.body, THRESHOLD) if context.body else 0
        ]
        return any(score >= THRESHOLD for score in scores)