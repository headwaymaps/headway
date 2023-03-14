import { IControl } from 'maplibre-gl';

export default class WrapperControl implements IControl {
  element: HTMLElement;
  children: IControl[];

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'headway-ctrl-wrapper';
    this.children = [];
  }

  pushChild(el: IControl) {
    this.children.push(el);
  }

  onAdd(map: maplibregl.Map): HTMLElement {
    for (const child of this.children) {
      this.element.appendChild(child.onAdd(map));
    }
    return this.element;
  }

  onRemove(map: maplibregl.Map): void {
    for (const child of this.children) {
      child.onRemove(map);
    }
    this.element.innerHTML = '';
  }
}
