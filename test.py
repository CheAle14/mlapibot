import praw
import toml 
import logging

handler = logging.StreamHandler()
handler.setLevel(logging.DEBUG)
for logger_name in ("praw", "prawcore"):
    logger = logging.getLogger(logger_name)
    logger.setLevel(logging.DEBUG)
    logger.addHandler(handler)

with open("settings.toml", "r") as f:
    settings = toml.load(f)

reddit = praw.Reddit(**settings['reddit'])

print("Logged in as", reddit.user.me())

# to check reddit API responses.