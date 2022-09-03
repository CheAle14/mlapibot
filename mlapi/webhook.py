import praw, logging, requests
import json
from praw.models import Submission, Comment

class WebhookSender:
    def __init__(self, webhookUrl, subreddit):
        self.WEBHOOK_URL = webhookUrl
        self.SubReddit = subreddit

    def getEmbed(self, title, desc, url, footer):
        embed = {}
        embed["title"] = title
        embed["description"] = desc
        if not url.startswith("http"):
            url = "https://old.reddit.com" + url
        embed["url"] = url
        if footer:
            embed["footer"] = {"text": footer}
        return embed


    def _sendWebhook(self, embed):
        if not self.WEBHOOK_URL:
            return
        data = {}
        data["username"] = "/r/" + self.SubReddit
        data["embeds"] = [embed]
        result = requests.post(self.WEBHOOK_URL, data=json.dumps(data), headers={"Content-Type": "application/json"})
        try:
            result.raise_for_status()
        except requests.exceptions.HTTPError as err:
            logging.error(err)
            if result.status_code >= 400:
                logging.error(result.text)
        else:
            logging.info("Payload delivered, %s", result.status_code)
    def sendSubmission(self, post: Submission, matches: str):
        embed = self.getEmbed(post.title, matches, post.permalink, post.author.name)
        self._sendWebhook(embed)

    def sendInboxMessage(self, message):
        title = "Reply" if isinstance(message, Comment) else "Inbox: " + message.subject
        embed = self.getEmbed(title, message.body, message.context, message.author.name)
        self._sendWebhook(embed)

    def sendRemovedComment(self, comment: praw.models.Comment):
        embed = self.getEmbed("Removed Comment", comment.submission.title, comment.submission.permalink, str(comment.score))
        self._sendWebhook(embed)
