<template><span hidden></span></template>
<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch } from 'vue';
import { Marker, Popup } from 'maplibre-gl';
import { useRouter } from 'vue-router';
import { baseMapPromise, map } from './BaseMap.vue';
import { previewText } from 'src/utils/previewText';
import { favorites, type Favorite } from 'src/models/Favorites';
const router = useRouter();
let markers: Marker[] = [];
let popup: Popup | undefined;
let selectedId: string | undefined;
let disposed = false;
function locationKey(favorite: Favorite) {
  return favorite.coordinates.join(',');
}
function showPopup(key: string) {
  popup?.remove();
  const group = favorites.value.filter((item) => locationKey(item) === key);
  const favorite = group[0];
  if (!map || !favorite || disposed) return;
  selectedId = key;
  const content = document.createElement('div');
  content.className = 'favorite-map-preview';
  const title = document.createElement('strong');
  title.textContent =
    group.length > 1
      ? `${group.length} favorites here`
      : previewText(favorite.name, 50);
  content.appendChild(title);
  if (group.length > 1) {
    const list = document.createElement('ul');
    list.className = 'favorite-map-choices';
    for (const item of group) {
      const row = document.createElement('li');
      const choice = document.createElement('button');
      choice.type = 'button';
      choice.textContent = previewText(item.name, 50);
      choice.title = item.name;
      choice.addEventListener('click', (event) => {
        event.stopPropagation();
        void router.push(`/favorites/${item.id}`);
      });
      row.appendChild(choice);
      if (item.description) {
        const description = document.createElement('p');
        description.textContent = previewText(item.description, 140);
        row.appendChild(description);
      }
      list.appendChild(row);
    }
    content.appendChild(list);
  } else if (favorite.description) {
    const description = document.createElement('p');
    description.textContent = previewText(favorite.description, 140);
    content.appendChild(description);
  }
  const current = new Popup({
    offset: 22,
    maxWidth: '260px',
    closeOnClick: true,
    focusAfterOpen: false,
  })
    .setLngLat(favorite.coordinates)
    .setDOMContent(content)
    .addTo(map);
  popup = current;
  current.on('close', () => {
    if (popup === current) {
      popup = undefined;
      selectedId = undefined;
    }
  });
}
function clear() {
  popup?.remove();
  markers.forEach((marker) => marker.remove());
  markers = [];
}
function render() {
  const previousSelection = selectedId;
  clear();
  if (!map || disposed) return;
  const groups = new Map<string, Favorite[]>();
  for (const favorite of favorites.value) {
    const key = locationKey(favorite);
    const group = groups.get(key) ?? [];
    group.push(favorite);
    groups.set(key, group);
  }
  for (const [key, group] of groups) {
    const favorite = group[0]!;
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'favorite-map-star';
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 32 32');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('focusable', 'false');
    const star = document.createElementNS(
      'http://www.w3.org/2000/svg',
      'polygon',
    );
    star.setAttribute(
      'points',
      '16,3 20,11.5 29.5,12.7 22.5,19.3 24.3,28.5 16,24 7.7,28.5 9.5,19.3 2.5,12.7 12,11.5',
    );
    svg.appendChild(star);
    button.appendChild(svg);
    if (group.length > 1) {
      const count = document.createElement('span');
      count.className = 'favorite-map-count';
      count.textContent = String(group.length);
      button.appendChild(count);
    }
    button.title =
      group.length > 1 ? `${group.length} favorites here` : favorite.name;
    button.setAttribute(
      'aria-label',
      group.length > 1
        ? `${group.length} favorites here`
        : `Open favorite: ${favorite.name}`,
    );
    button.addEventListener('click', (event) => {
      event.stopPropagation();
      showPopup(key);
      if (group.length === 1) void router.push(`/favorites/${favorite.id}`);
    });
    markers.push(
      new Marker({ element: button })
        .setLngLat(favorite.coordinates)
        .addTo(map),
    );
  }
  if (previousSelection) showPopup(previousSelection);
}
watch(favorites, render, { deep: true });
onMounted(async () => {
  await baseMapPromise;
  render();
});
onBeforeUnmount(() => {
  disposed = true;
  clear();
});
</script>
<style>
.favorite-map-star {
  border: none;
  padding: 3px;
  background: transparent;
  width: 38px;
  height: 38px;
  cursor: pointer;
}
.favorite-map-count {
  position: absolute;
  right: -3px;
  top: -3px;
  background: white;
  color: #333;
  border-radius: 10px;
  min-width: 18px;
  padding: 0 4px;
  font-size: 12px;
  font-weight: bold;
  box-shadow: 0 1px 3px #0006;
}
.favorite-map-choices {
  list-style: none;
  padding: 0;
  margin: 8px 0 0;
  max-height: 240px;
  overflow-y: auto;
}
.favorite-map-choices li {
  padding: 8px 0;
  border-top: 1px solid #ddd;
}
.favorite-map-choices button {
  background: transparent;
  border: none;
  padding: 0;
  text-align: left;
  color: #1565c0;
  font: inherit;
  cursor: pointer;
  overflow-wrap: anywhere;
}
.favorite-map-star svg {
  display: block;
  width: 100%;
  height: 100%;
  fill: #ffdf00;
  stroke: #ffdf00;
  stroke-width: 3;
  stroke-linejoin: round;
  filter: drop-shadow(0 1px 2px #0008);
}
.favorite-map-star:focus-visible {
  outline: 3px solid #1976d2;
}
.favorite-map-preview {
  padding: 4px 14px 4px 2px;
  font-size: 14px;
  overflow-wrap: anywhere;
}
.favorite-map-preview strong {
  display: block;
  font-size: 15px;
}
.favorite-map-preview p {
  margin: 6px 0 0;
  white-space: normal;
}
</style>
