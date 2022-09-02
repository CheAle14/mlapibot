from re import sub
from typing import Dict
from xml.etree.ElementInclude import include
from zoneinfo import ZoneInfo
from datetime import datetime
from dateutil.parser import parse
from unicodedata import name
from praw.models import Submission
from praw import Reddit
import requests
import zoneinfo
pst = ZoneInfo("US/Pacific")
def parseDate(dateStr):
    date = parse(dateStr)
    return date.astimezone(pst)


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
        try:
            self.createdAt = parseDate(json["created_at"])
        except:
            self.createdAt = None
        try:
            self.updatedAt = parseDate(json["updated_at"])
        except:
            self.updatedAt = None
        try:
            self.monitoringAt = parseDate(json["monitoring_at"])
        except:
            self.monitoringAt = None
        try:
            self.resolvedAt = parseDate(json["resolved_at"])
        except:
            self.resolvedAt = None
        self.impact = json["impact"]
        self.shortlink = json["shortlink"]
        self.startedAt = parseDate(json["started_at"])
        self.page_id = json["page_id"]
        self.updates = [StatusIncidentUpdate(x) for x in json["incident_updates"]]

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
    def __init__(self, root):
        self.root = root

    def _get(self, path):
        resp = requests.get(self.root + path)
        resp.raise_for_status()
        return resp.json()

    def summary(self):
        return StatusSummary(self._get("/summary.json"))

    def incidents(self):
        return StatusPageIncident(self._get("/incidents.json"))

    def incident(self, id : str):
        resp = self._get("/incidents/{0}.json".format(id))
        return resp

class StatusReporter:
    def __init__(self, api : StatusAPI):
        self.postId = None
        self.incidentsTracked : Dict[str, StatusIncident] = {}
        self.lastUpdated : datetime | None = None
        self.api = api
    
    def getPost(self, reddit : Reddit):
        if self.postId:
            return Submission(reddit, self.postId)
        else:
            return None

    def shouldUpdate(self):
        return self.lastUpdated is None or (datetime.now() - self.lastUpdated).total_seconds() > 300

    def checkStatus(self):
        if not self.shouldUpdate(): return False
        summary = self.api.summary()

        for inc in summary.incidents:
            self.add(inc)

        if len(self.incidentsTracked) == 0:
            return False
        
        # There are incidents that should be updated.
    
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
        for key in self.incidentsTracked:
            if key not in updated:
                resp = self.api.incident(key)
                self.add(resp)
        



    def getBody(self):
        s = ""
        for id, incident in self.incidentsTracked.items():
            s += incident.getBody()
            s += "\r\n\r\n---\r\n\r\n"
        return s