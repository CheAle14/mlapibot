{% extends "base.md" %}

{% block content %}
The image(s) you've submitted appear to contain a common repost concerning using Discord's substition syntax with Tenor gifs.  
This works through the [sed-like](https://en.wikipedia.org/wiki/Sed) syntax `s/text/replacement`, which will cause the first instance of `text` to be replaced with `replacement` in your last message. Since that message is a Tenor gif starting `https://tEnor.com/...`, typing `s/e/x` causes that first `e` to be replaced with `x`, changing the URL to point to a different domain that has been made to respond with the image then shown.

{% endblock content %}
