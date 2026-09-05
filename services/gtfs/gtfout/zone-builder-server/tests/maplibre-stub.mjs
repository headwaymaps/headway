// Stands in for maplibre-gl, which the page imports from a CDN.
//
// The projection is linear and exactly invertible, so a test can name a
// rectangle in screen pixels and predict the bbox the page will ask the server
// for. Real Web Mercator would make every expected coordinate a magic number
// without testing anything the page is responsible for.
//
// x = lon * 100, y = (50 - lat) * 100

export class Map {
  constructor(options) {
    this.options = options;
    this.handlers = {};
    this.sources = {};
    this.layers = [];
    this.controls = [];
    this.fitted = [];
    this.dragPan = {
      enabled: true,
      enable: () => (this.dragPan.enabled = true),
      disable: () => (this.dragPan.enabled = false),
    };
    // The page reads this at module scope and attaches its drag listeners to it.
    this.canvasContainer = document.createElement('div');
    document.getElementById('map')?.appendChild(this.canvasContainer);
  }

  addControl(control, position) {
    this.controls.push({ control, position });
  }

  getCanvasContainer() {
    return this.canvasContainer;
  }

  on(event, handler) {
    (this.handlers[event] ||= []).push(handler);
  }

  emit(event, ...args) {
    for (const handler of this.handlers[event] || []) handler(...args);
  }

  project([lng, lat]) {
    return { x: lng * 100, y: (50 - lat) * 100 };
  }

  unproject([x, y]) {
    return { lng: x / 100, lat: 50 - y / 100 };
  }

  addSource(id, source) {
    this.sources[id] = { ...source, setData: (data) => (this.sources[id].data = data) };
  }

  getSource(id) {
    return this.sources[id];
  }

  addLayer(layer) {
    this.layers.push(layer);
  }

  fitBounds(bounds, options) {
    this.fitted.push({ bounds, options });
  }
}

export class NavigationControl {}
export class ScaleControl {}
