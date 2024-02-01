from typing import Dict

from typing_extensions import Any, Self

from mlapi.models.scams import BaseScamChecker, ScamContext
from mlapi.ocr import FUNCTIONS


class FunctionScamChecker(BaseScamChecker):
    def __init__(self, name, ignore_self_posts, template, report, blacklist, function):
        super().__init__(name,ignore_self_posts, template, report, blacklist)
        self.function = function
    
    @staticmethod
    def from_json(json: Dict[str, Any]) -> Self:
        return FunctionScamChecker(
            json.get("name"),
            json.get("ignore_self_posts", False),
            json.get("template", "default"),
            json.get("report", False),
            json.get("blacklist", []),
            json.get("function")
        )
    
    def matches(self, context: ScamContext, THRESHOLD: float) -> float:
        if self.function not in FUNCTIONS:
            raise ValueError(f"Invalid function name '{self.function}', expected:", FUNCTIONS.keys())
        f = FUNCTIONS.get(self.function, None)
        if f is not None and f(context):
            return 1.0
        return 0.0