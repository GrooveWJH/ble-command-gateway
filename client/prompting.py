from typing import Any

from InquirerPy.resolver import prompt, question_mapping


def ask_list(message: str, choices: list[Any], default: Any | None = None) -> Any:
    question: dict[str, Any] = {
        "type": "list",
        "name": "value",
        "message": message,
        "choices": choices,
    }
    if default is not None:
        question["default"] = default
    return prompt([question])["value"]


def ask_text(message: str, default: str = "") -> str:
    return str(
        prompt(
            [
                {
                    "type": "input",
                    "name": "value",
                    "message": message,
                    "default": default,
                }
            ]
        )["value"]
    )


def ask_secret(message: str, default: str = "") -> str:
    question_type = "password" if "password" in question_mapping else "secret"
    return str(
        prompt(
            [
                {
                    "type": question_type,
                    "name": "value",
                    "message": message,
                    "default": default,
                }
            ]
        )["value"]
    )
