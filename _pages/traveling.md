---
layout: default
title: Traveling
permalink: /traveling/
nav: false
---

<!-- Inspired by https://www.stefanocottafavi.com/leaflet-jekyll/ -->

<link rel="stylesheet" href="/assets/css/leaflet.min.css" />
<script src="/assets/js/leaflet.min.js"></script>

<link rel="stylesheet" href="/assets/css/leaflet-gesture-handling.min.css" />
<script src="/assets/js/leaflet-gesture-handling.min.js"></script>

<script src="/assets/js/leaflet-providers.min.js"></script>

Discover my travel journey on this interactive map! Each marker pinpoints a city I've explored. Check out where I've been and stay tuned for more destinations and upcoming travel photos!

<style>
#map {
    width: 100%;
    height: 500px;
}
</style>

<div id="map"></div>

<script>
    var map = new L.Map("map", {
        center: new L.LatLng(35, 0),
        zoom: 2,
        gestureHandling: true
    });

    // Define smaller marker icon
    var mark = L.icon({
        iconUrl: '/assets/icons/marker-icon.png',
        iconSize: [16, 24], // Smaller size (width, height)
        iconAnchor: [8, 24], // Adjust anchor to match new size
        popupAnchor: [0, -24] // Adjust popup position
    });

    L.tileLayer('https://tiles.stadiamaps.com/tiles/stamen_toner_lite/{z}/{x}/{y}{r}.{ext}', {
        minZoom: 0,
        maxZoom: 20,
        attribution: '&copy; <a href="https://www.stadiamaps.com/" target="_blank">Stadia Maps</a> &copy; <a href="https://www.stamen.com/" target="_blank">Stamen Design</a> &copy; <a href="https://openmaptiles.org/" target="_blank">OpenMapTiles</a> &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
        ext: 'png'
    }).addTo(map);

    var bounds = L.latLngBounds([
        {% for place in site.data.traveling %}
            {% if place.latitude and place.longitude %}
                [{{place.latitude}}, {{place.longitude}}],
            {% endif %}
        {% endfor %}
    ]);

    {% for place in site.data.traveling %}
        {% if place.latitude and place.longitude %}
            L.marker([{{place.latitude}}, {{place.longitude}}], { title: '{{place.city}}', icon: mark }).addTo(map);
        {% endif %}
    {% endfor %}

    // Fit map to bounds if there are valid bounds
    if (bounds.isValid()) {
        map.fitBounds(bounds, { padding: [50, 50] });
    }
</script>