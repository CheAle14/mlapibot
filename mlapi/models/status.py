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
    "embed": ["link"],
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
    "android": [],
    "canary": [],
    "ptb": []
}


def isKeyWord(word : str):
    # Normalise the word.
    word = word.lower()
    # Remove 's' from end.
    if word.endswith('s'):
        word = word[:-1]
    
    # Remove -ing from end.
    if word.endswith('ing'):
        word = word[:-3]

    if word in IGNORE_WORDS: return False

    for key, ls in KEYWORDS.items():
        if key == word: return True
        if word in ls: return True
    return False



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
        if self._cachekeywords:
            return self._cachekeywords
        words = {}
        for word in self.name.split():
            if word not in words and isKeyWord(word):
                words[word] = self.name
        for updt in self.updates:
            for word in updt.body.split():
                if word not in words and isKeyWord(word):
                    words[word] = updt.body
        for comp in self.components:
            for word in comp.name.split():
                if word not in words and isKeyWord(word):
                    words[word] = comp.name
        self._cachekeywords = words
        return words


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
        self.postId = None
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
        self.postId = data["postId"]
        self.lastUpdated = parseUtc(data["lastUpdated"])
        self.lastSent = parseUtc(data["lastSent"])
        self.incidentsTracked = {}
        for x in data["incidents"]:
            self.incidentsTracked[x] = None
        self.fetchAllIncidents()

    def save(self, path = "status.json"):
        if self.postId:
            data = {
                "postId": self.postId,
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
        if self.postId:
            return (Submission(subreddit._reddit, self.postId), False)
        else:
            post = subreddit.submit(title=self.getTitle(), selftext=self.getBody(), send_replies=False)
            if subreddit.display_name == "mlapi":
                self.replyDebugInfo(post)
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

    def checkStatus(self, subreddit : Subreddit) -> Union[Submission, None]:
        if not self.shouldUpdate(): return None
        logging.info("Fetching Discord status...")
        summary = self.api.summary()
        logging.info("Fetched with " + str(summary.status.indicator) + ": " + str(summary.status.description) + "; incidents: " + str(len(summary.incidents)))
        self.lastUpdated = datetime.now(utc)

        rtn_post = None

        try:
            for inc in summary.incidents:
                self.add(inc)

            if len(self.incidentsTracked) > 0:
                if self.shouldSend():
                    rtn_post = self.sendToPost(subreddit)
                elif self.areAllResolved() and self.postId is not None:
                    rtn_post = self.sendToPost(subreddit)
                    self.incidentsTracked = {}
                    self.lastSent = None
                    self.postId = None
        finally:
            self.save()    
        return rtn_post

    def sendToPost(self, subreddit : Subreddit) -> Union[Submission, None]:
        (post, newlyCreated) = self.getOrCreateSubmission(subreddit)
        if newlyCreated:
            self.postId = post.id
        else:
            post.edit(body=self.getBody())
        self.lastSent = datetime.now(utc)
        if newlyCreated: return post
        return None
        


    
    def setPost(self, submission : Submission):
        self.postId = submission.id

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
        

    def getTitle(self):
        s = "Discord "
        highestState = ""
        isoutage = False
        involves = []
        for id, incident in self.incidentsTracked.items():
            if incident.impact == "critical":
                highestState = "Critical"
            elif incident.impact == "major" and highestState != "critical":
                highestState = "Major"
            elif incident.impact == "minor" and (highestState != "major" and highestState != "critical"):
                highestState = "Minor"
            if "outage" in incident.name.lower():
                isoutage = True
            for x in incident.components:
                if x.name not in involves: involves.append(x.name)
        if highestState != "":
            s += highestState + " "
        s += ", ".join(involves)
        if isoutage:
            s += " outage"
        else:
            s += " issue"
        return s

    def getBody(self):
        s = ""
        for id, incident in self.incidentsTracked.items():
            s += incident.getBody()
            s += "\r\n\r\n---\r\n\r\n"
        return s