{% extends "base.md" %}

{% block content %}
This post appears to be asking why Discord asks for permission to receive keystrokes in any application.  
This question has been answered by a Discord employee [in this comment](https://reddit.com/r/discordapp/comments/haygfd/why_is_discord_asking_permission_to_record_all_of/fv6gs2e/):

> We require accessibility permissions for systemwide push-to-talk to work. If you disable this permission, Discord will still function just fine, you’ll just need to open the app for your PTT hot key to function.

{% endblock content %}
