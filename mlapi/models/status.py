from typing import Dict, Union
from pytz import timezone
import pytz
from datetime import datetime
from dateutil.parser import parse
from praw.models import Submission, Subreddit, Comment
import requests
import json
import os
import logging
import re
#import yake

pst = timezone("US/Pacific")
utc = pytz.utc

def convert(datetime : datetime, tz):
    if datetime.tzinfo:
        return datetime.astimezone(tz)
    else:
        return tz.localize(datetime)

def parseDate(dateStr):
    if dateStr is None: return None
    return convert(parse(dateStr), pst)
def parseUtc(dateStr):
    return convert(parse(dateStr), utc)
def now_utc():
    return convert(datetime.now(), utc)

IGNORE_WORDS = [
    "a", "we", "the", "we've", "has", "identified", "we're", "being"
]
KEYWORDS = {
    "emote": ["emoji"],
    "message send": [],
    "attachment": ["file", "media"],
    "embed": ["link", "preview"],
    "purchase": [],
    "ban": [],
    "kick": [],
    "prune": [],
    "leave": ["left"],
    "spam": [],
    "latency": ["lag"],
    "guild": ["server"],
    "cdn": ["cloudflare"],
    "edit": ["modify", "update", "change"],
    "delete": ["remove"],
    "voice": ["vc"],
    "ios": ["iphone"],
    "android": ["126.20", "126.21", "v126", "v126.20", "v126.21"],
    "canary": [],
    "ptb": [],
    "client": [],
    "outage": ["not respond", "not load", "down", "unavailable", "error"],
    "member list": ["members"],
    "user": ["member"],
    "application": ["bot"],
    "purchase": ["subscription", "tax"],
    "login": ["auth", "authentication", "MFA", "2FA"]
}
#kw_extractor = yake.KeywordExtractor()


def isKeywordPresent(word: str, text: str) -> bool:
    return re.search(f"\\b{word}(s|er|ing)?\\b", text, re.IGNORECASE)



class Status:
    def __init__(self, json):
        self.indicator = json["indicator"]
        self.description = json["description"]

class StatusPage:
    def __init__(self, json):
        self.id = json["id"]
        self.name = json["name"]
        self.url = json["url"]
        self.updatedAt = parseDate(json["updated_at"])

class StatusComponent:
    def __init__(self, json):
        self.id = json["id"]
        self.name = json.get("name")
        self.status = json.get("status")
        self.description = json.get("description")

class StatusIncidentUpdate:
    def __init__(self, json):
        self.id = json["id"]
        self.status = json["status"]
        self.body = json["body"]
        try:
            self.createdAt = parseDate(json["created_at"])
        except:
            self.createdAt = None

class StatusIncident:
    def __init__(self, json):
        self.id = json["id"]
        self.name = json["name"]
        self.status = json["status"]

        self.createdAt = parseDate(json.get("created_at", None))
        self.updatedAt = parseDate(json.get("updated_at", None))
        self.monitoringAt = parseDate(json.get("monitoring_at", None))
        self.resolvedAt = parseDate(json.get("resolved_at", None))
        self.impact = json["impact"]
        self.shortlink = json["shortlink"]
        self.startedAt = parseDate(json.get("started_at", None))
        self.page_id = json["page_id"]
        self.updates = [StatusIncidentUpdate(x) for x in json.get("incident_updates", [])]
        self.components = [StatusComponent(x) for x in json.get("components", [])]

        self._cachekeywords = None

    def getKeywords(self):
        lines = []
        if self.name:
            lines.append(self.name)
        #for component in self.components:
        #    lines.append(component.name)
        for update in self.updates:
            if update.status == "resolved": continue
            lines.append(update.body)
        nl_keywords = [] # [kw[0].lower() for kw in kw_extractor.extract_keywords("\r\n".join(lines)) if kw[1] <= KEYWORD_THRESHOLD]
        
        for comp in self.components:
            lines.append(comp.name)
            if comp.description:
                lines.append(comp.description)

        hardcoded_words = []
        fulltext = "\n".join(lines).lower()
        for key, array in KEYWORDS.items():
            if key not in hardcoded_words and isKeywordPresent(key, fulltext):
                hardcoded_words.append(key)
            for wd in array:
                if wd not in hardcoded_words and isKeywordPresent(wd, fulltext):
                    hardcoded_words.append(wd)

        return (nl_keywords, hardcoded_words)


    def getTitle(self):
        s = "Discord "
        if self.impact == "critical":
            s += "Outage"
        else: s += "Status"
        s += ": " + self.name

    def getBody(self):
        body = "[" + self.name + "](" + self.shortlink + ")  \r\n"
        hasExisting = {}
        sections = []
        for update in reversed(self.updates):
            section = ""
            if update.status in hasExisting:
                section += "**Update**"
            else:
                hasExisting[update.status] = True
                section += "**" + update.status[0].upper() + update.status[1:] + "**  "
            section += "\r\n"
            for line in update.body.split('\n'):
                section += "> " + line + "  \r\n"

            section += "\r\n"
            if update.createdAt:
                section += "> ^(" + update.createdAt.strftime("%b %d, %Y - %H:%M %Z") + ")"
                section += "\r\n"
            sections.append(section)

        return body + "\r\n".join(reversed(sections))

    def __str__(self):
        return f"{self.id} {self.name} {self.status} {self.startedAt} {self.resolvedAt}"




class StatusSummary:
    def __init__(self, json):
        self.page = StatusPage(json["page"])
        self.components = json["components"]
        self.status = Status(json["status"])
        self.incidents = [StatusIncident(x) for x in json["incidents"]]

    def __str__(self):
        s = self.status.description
        for inc in self.incidents:
            s += "\r\n" + str(inc)
        return s

class StatusPageIncident:
    def __init__(self, json):
        self.page = StatusPage(json["page"])
        self.incidents = [StatusIncident(x) for x in json["incidents"]]
    def __str__(self):
        s = str(self.page.updatedAt)
        for inc in self.incidents:
            s += "\r\n" + str(inc)
        return s


class StatusAPI:
    def __init__(self, root, debugStatus = None):
        self.root = root
        self._debugStatus = debugStatus

    def _get(self, path):
        resp = requests.get(self.root + path)
        resp.raise_for_status()
        return resp.json()

    def summary(self):
        if self._debugStatus: 
            sum = StatusSummary(self._debugStatus)
            self._debugStatus = None
            return sum
        return StatusSummary(self._get("/summary.json"))

    def incidents(self):
        return StatusPageIncident(self._get("/incidents.json"))

    def incident(self, id : str):
        try:
            resp = StatusIncident(self._get("/incidents/{0}.json".format(id)))
        except requests.exceptions.HTTPError as e:
            logging.error(e, exc_info=1)
            logging.error("Failed to fetch incident with id " + id)
            resp = None
        return resp

class StatusReporter:
    def __init__(self, api : StatusAPI):
        self.posts = {}
        self.incidentsTracked : Dict[str, StatusIncident] = {}
        self.lastUpdated : datetime = None
        self.lastSent : datetime = None
        self.api = api

    def load(self, path = "status.json"):
        try:
            with open(path, "r") as f:
                data = json.load(f)
        except FileNotFoundError:
            return # nothing to load.
        self.posts = data["posts"]
        self.lastUpdated = parseUtc(data["lastUpdated"])
        self.lastSent = parseUtc(data["lastSent"])
        self.incidentsTracked = {}
        for x in data["incidents"]:
            self.incidentsTracked[x] = None
        self.fetchAllIncidents()

    def save(self, path = "status.json"):
        if len(self.posts) > 0:
            data = {
                "posts": self.posts,
                "lastUpdated": self.lastUpdated.isoformat(),
                "lastSent": self.lastSent.isoformat(),
                "incidents": []
            }
            for x, v in self.incidentsTracked.items():
                data["incidents"].append(x)
            with open(path, "w") as f:
                json.dump(data, f)
        else:
            try:
                os.remove(path)
            except: pass

        
    def replyDebugInfo(self, submission : Submission):
        lines = ["# Debug Information"]

        for k, incident in self.incidentsTracked.items():
            lines.append("Id: " + k)
            lines.append("Name: " + incident.name)
            lines.append("Impact: " + incident.impact)
            lines.append("Status: " + incident.status)

            if len(incident.updates) > 0:
                lines.append("Updates:\r\n")
                for upd in incident.updates:
                    lines.append("- " + upd.id + ": " + upd.status + "; " + upd.body)
                lines.append("")

            if len(incident.components) > 0:
                lines.append("Components:\r\n")
                for com in incident.components:
                    lines.append("- " + com.id + ": " + com.name)
                lines.append("")


            lines.append("-----")
            lines.append("")

        body = "  \r\n".join(lines)
        rep : Comment = submission.reply(body=body)
        try:
            rep.mod.distinguish(sticky=True)
        except: pass
        
        
    
    def getOrCreateSubmission(self, subreddit : Subreddit):
        postId = self.posts.get(subreddit.fullname, None)
        if postId:
            return (Submission(subreddit._reddit, postId), False)
        else:
            post = subreddit.submit(title=self.getTitle(), selftext=self.getBody(), send_replies=False)
            if subreddit.display_name == "mlapi":
                self.replyDebugInfo(post)
            self.posts[subreddit.fullname] = post.id
            return (post, True)

    def shouldUpdate(self):
        return self.lastUpdated is None or (datetime.now(utc) - self.lastUpdated).total_seconds() > 300

    def shouldSend(self):
        if self.lastSent is None: return True
        for id, incident in self.incidentsTracked.items():
            if incident.updatedAt and incident.updatedAt > self.lastSent:
                return True
        return False

    def areAllResolved(self):
        self.fetchAllIncidents()
        for key, value in self.incidentsTracked.items():
            if value.resolvedAt is None: return False
        return True

    def checkStatus(self, testSubreddit : Subreddit, mainSubreddit : Subreddit) -> Union[Submission, None]:
        if not self.shouldUpdate(): return None
        logging.info("Fetching Discord status...")
        summary = self.api.summary()
        logging.info("Fetched with " + str(summary.status.indicator) + ": " + str(summary.status.description) + "; incidents: " + str(len(summary.incidents)))
        self.lastUpdated = datetime.now(utc)

        rtn_post = None

        try:
            anyMajorOrMore = False
            inc : StatusIncident = None
            (highestState, isoutage, involves, name) = self.getImpacts()
            anyMajorOrMore = isoutage or (highestState in ['Critical', 'Major'])

            if len(self.incidentsTracked) > 0:
                logging.info(f"{len(self.incidentsTracked)} incidents tracked; with {anyMajorOrMore} major+")
                sendTo = [testSubreddit]
                if anyMajorOrMore:
                    sendTo.append(mainSubreddit)
                for sendSub in sendTo:
                    if self.shouldSend():
                        logging.info(f"Sending/updating post in /r/{sendSub.display_name}")
                        rtn_post = self.sendToPost(sendSub)
                    elif self.areAllResolved() and self.posts.get(sendSub.fullname, None) is not None:
                        rtn_post = self.sendToPost(sendSub)
                        self.posts.pop(sendSub.fullname)
                if self.areAllResolved():
                    logging.info(f"All incidents are resolved.")
                    self.incidentsTracked = {}
                    self.lastSent = None
                    self.posts.clear()
            else:
                logging.info("No incidents tracked.")
        finally:
            self.save()    
        return rtn_post

    def sendToPost(self, subreddit : Subreddit) -> Union[Submission, None]:
        (post, newlyCreated) = self.getOrCreateSubmission(subreddit)
        if not newlyCreated:
            post.edit(body=self.getBody())
        self.lastSent = datetime.now(utc)
        if newlyCreated: return post
        return None
        
    def isTracked(self, incident : StatusIncident):
        return incident.id in self.incidentsTracked
    
    def add(self, incident : StatusIncident):
        self.incidentsTracked[incident.id] = incident

    def fetchAllIncidents(self):
        updated = []
        resp = self.api.incidents()
        for x in resp.incidents:
            if x.id in self.incidentsTracked:
                self.add(x)
                updated.append(x.id)
        drop = []
        for key in self.incidentsTracked:
            if key not in updated:
                resp = self.api.incident(key)
                if resp:
                    self.add(resp)
                else: # not found? so remove
                    drop.append(key)
        for x in drop:
            self.incidentsTracked.pop(x, None)
        
    def getImpacts(self):
        highestState = ""
        isoutage = False
        involves = []
        name = ""
        for id, incident in self.incidentsTracked.items():
            if incident.impact == "critical":
                highestState = "Critical"
                name = incident.name
            elif incident.impact == "major" and highestState != "critical":
                highestState = "Major"
                name = incident.name
            elif incident.impact == "minor" and (highestState != "major" and highestState != "critical"):
                highestState = "Minor"
                name = incident.name
            if name == "":
                name = incident.name
            if "outage" in incident.name.lower():
                isoutage = True
            for x in incident.components:
                if x.name not in involves: involves.append(x.name)
        return (highestState, isoutage, involves, name)

    def getTitle(self):
        s = ""
        (highestState, isoutage, involves, name) = self.getImpacts()
        if highestState != "":
            s += highestState + " "
        s += ", ".join(involves)
        if isoutage:
            s += " outage"
        else:
            s += " issue"
        
        if name != "":
            s += ": " + name
        return s

    def getBody(self):
        s = ""
        for id, incident in self.incidentsTracked.items():
            s += incident.getBody()
            s += "\r\n\r\n---\r\n\r\n"
        return s
    

if __name__ == "__main__":
    api = StatusAPI("https://discordstatus.com/api/v2")

    incidents = api.incidents()
    seen_keywords = {}
    seen_hardcoded = {}
    for inc in incidents.incidents:
        nl, hc  = inc.getKeywords()
        for word in nl:
            seen_keywords[word] = seen_keywords.get(word, 0) + 1
        for word in hc:
            seen_hardcoded[word] = seen_hardcoded.get(word, 0) + 1
    for key, value in seen_keywords.items():
        if value == 1: continue
        print(key, "=", value)
    for key, value in seen_hardcoded.items():
        if value == 1: continue
        print(key, ":", value)