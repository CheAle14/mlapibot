{% extends "base.md" %}
{% block content %}
The image(s) you've submitted appear to contain a common DM scam. DM scams like these usually come from compromised user or bot accounts.

When looking at a possible scam from either a bot account or user account, always consider if they:

- Are new, unfamiliar, unverified (in the case of bot accounts) or contacting you unprompted
- Are not from Discord: not through email from them, or from a [System-tagged account](https://support.discord.com/hc/articles/360036118732)
- Have poor grammar, spelling or misuse punctuation or capitalisation
- Offer things that are 'too good to be true'

Official Discord gifts use the `discord.gift` domain, and will generate a special embed, [shown in this image](https://imgur.com/Xsy1zdE). These gifts can be claimed entirely **in-app**, by pressing the Accept button that generates **inside the embed**, so you should not trust any without that button, or whose buttons are outside the embed or take you out of the app.

---

To get rid of this bot, you can:

- Block it
- [Report it to Discord](https://dis.gd/howtoreport)

If these types of bots are repeatedly sending you messages, you can:

- Use Mutual Servers to determine the server(s) they share with you, and disable Direct Messages from server members for those servers.
- If you cannot find any common servers, you can disable DMs from all servers under your User Settings

If your account is the one sending this message, then it means your account has been compromised. If you...

- ... downloaded and executed malware: You should try and use a different device entirely to change your password (e.g. your phone). You should then [follow these steps](https://support.discord.com/hc/articles/115004307527--Windows-Corrupt-Installation) to fully uninstall Discord, run a complete anti-virus scan, and then re-install Discord. If your account is compromised again when logging in afterwards, you may need to factory reset your computer.
- ... entered your password into a malicious/fake website: You should change your password.
- ... scanned a QR code in-app and then authorized the login: You should change your password.
- ... did something else: You should change your password.

You should also check your Authorised Apps (under User Settings) to ensure that no suspicious applications have been added (remove any that you don't recognise or didn't add, especially those with "Join servers for you"). If you had payment information saved, you should also double check to ensure no gifts were bought whilst your account was compromised.

{% endblock content %}
