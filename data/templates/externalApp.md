{% extends "base.md" %}

{% block content %}

This post could be asking about a bot somehow being in a server without being added.  
If so, it is likely one of two options:

1. It is a bot that has been installed onto a user account, allowing the user to use the bot in DMs and in servers that allow it.  
   You can disable the **Use External Apps** permission to stop this.  
   You can see which user initiated the command by right clicking on the bot's message and selecting 'View Interaction Info'.
2. It is a webhook that was added by someone with the Manage Webhooks permission.  
   You can remove the webhook under the channel's Integration settings.

{% endblock content %}
