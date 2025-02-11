{% extends "base.md" %}
{% block content %}
The image(s) you've submitted appear to show the 'New login location detected, please check your e-mail' error when logging in.

This is showing because:

1. the IP address you are attempting to login from is different to ones that you have used in the past; and
2. your account does not have 2FA enabled.

Discord should have sent you an email that you can use to verify/confirm that you are the one logging in. Check your inbox and spam folders.

If you no longer have access to this email, you will need to contact your email provider to try and restore/recover your access to it.

For further help, you may attempt to contact Discord's support at [dis.gd/support](https://dis.gd/support).

{% endblock content %}
