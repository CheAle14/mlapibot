{% extends "base.md" %}

{% block content %}
I'm responding because I think this post is about Two-Factor Authentication; the following are some frequently asked questions that may help help:

### How to setup 2FA?

[Setting up Two-Factor Authentication](https://support.discord.com/hc/articles/219576828-Setting-up-Two-Factor-Authentication). Do not forget to download and write down your backup codes.

### I can't login because of 2FA

There are three ways to login to a 2FA protected account:

- Using a 6-digit time-based code generated from the authenticator app installed on your phone when you first setup 2FA;
- Using an 8-digit backup code that you should've downloaded or otherwise saved when you setup 2FA. Note that these codes are one-time use, though you can generate new ones at any time; or
- If you enabled the SMS backup authentication option, then by SMS whilst trying to login on desktop or browser, or desktop mode in your phone's browser. This option is not the same thing as having a verified phone number on the account, and the option is disabled by default.

If a code cannot be gotten through any of the above means, then your account is lost and cannot be recovered. [Discord does not bypass or remove 2FA](https://support.discord.com/hc/articles/115001221072-Lost-Two-Factor-Codes)

### I can't login because I've forgotten my password

Resetting your password by email requires either a 6-digit code or 8-digit backup code. As with logging in, if you're unable to get one of these codes, your account is likely lost.

### I've been locked out, how do I delete the account?

You can submit an Account Deletion Request at [dis.gd/support](https://dis.gd/support)

{% endblock content %}
