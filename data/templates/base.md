Hi!

{% block content %}
{% endblock %}

---

{% block footer %}

{%if removal_reason %}

{{ removal_reason }} {% if imgur_url %} ^[[OCR]]({{ imgur_url }}) {% endif %}

{% else %}

^(I am a bot; if this comment was made in error, please correct and downvote me.) {% if imgur_url %} ^[[OCR]]({{ imgur_url }}) {% endif %}

{% endif %}
{% endblock footer %}
