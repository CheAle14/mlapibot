import praw, logging, requests
import json
from praw.models import Submission

WEBHOOK_URL = ""
with open("webhook.txt", "r") as f:
    WEBHOOK_URL = f.read()

def getEmbed(title, desc, url, footer):
    embed = {}
    embed["title"] = title
    embed["description"] = desc
    embed["url"] = "https://old.reddit.com" + url
    if footer:
        embed["footer"] = {"text": footer}
    return embed


def _sendWebhook(embed):
    if not WEBHOOK_URL:
        logging.warn("Not sending webhook because URL is empty")
        return
    data = {}
    data["content"] = "From Reddit"
    data["username"] = "/r/DiscordApp"
    data["embeds"] = [embed]
    result = requests.post(WEBHOOK_URL, data=json.dumps(data), headers={"Content-Type": "application/json"})
    try:
        result.raise_for_status()
    except requests.exceptions.HTTPError as err:
        logging.error(err)
    else:
        logging.info("Payload delivered, %s", result.status_code)
def sendSubmission(post: Submission, matches: str):
    embed = getEmbed(post.title, matches, post.permalink, post.author.name)
    _sendWebhook(embed)

def sendInboxMessage(message):
    embed = getEmbed("Inbox Message", message.body, message.context, message.author.name)
    _sendWebhook(embed)
