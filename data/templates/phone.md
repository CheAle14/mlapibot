{% extends "base.md" %}

{% block content %}
Your post appears to be about the "Verification Required" prompt, which is connected to Discord's anti-abuse system, which attempts to use CAPTCHAs, email and phone verification to target abusive behaviour of their platform, like that of spam bots.  
If you are having issues verifying, try [contacting Discord support](https://dis.gd/support), however, in most cases support are unable to lift the requirement to verify; if you only have the ability to verify by phone, then you can either verify by phone or cease using that account - there's no way around the requirement.

If your phone number has been flagged as invalid and was typed correctly, then this likely means Discord believes it is a VOIP, burner or landline number, or was blacklisted due to being connected to a previously terminated account. There doesn't appear to be any way to appeal this, which means you'll either need to use someone else's phone number, or cease using that account.

For phone verification, note that Discord only allows a number to be connected to one account at a time, and has a cooldown before a number can be placed on a new account after being used to verify.

{% endblock content %}
