type IDType = 'node' | 'way' | 'relation';

export default class OSMID {
  idType: IDType;
  idNumber: number;

  constructor(idType: IDType, idNumber: number) {
    this.idType = idType;
    this.idNumber = idNumber;
  }

  static deserialize(serializedId: string): OSMID {
    const [typeName, idString] = serializedId.split('/');
    const id = Number(idString);
    if (!id) {
      throw new Error(`invalid OSM ID number: ${serializedId}`);
    }

    switch (typeName) {
      case 'node':
        return OSMID.node(id);
      case 'way':
        return OSMID.way(id);
      case 'relation':
        return OSMID.relation(id);
      default:
        throw new Error(`invalid OSM ID type: ${serializedId}`);
    }
  }

  static node(id: number): OSMID {
    return new OSMID('node', id);
  }
  static way(id: number): OSMID {
    return new OSMID('way', id);
  }
  static relation(id: number): OSMID {
    return new OSMID('relation', id);
  }

  isNode(): boolean {
    return this.idType == 'node';
  }
  isWay(): boolean {
    return this.idType == 'way';
  }
  isRelation(): boolean {
    return this.idType == 'relation';
  }
}
