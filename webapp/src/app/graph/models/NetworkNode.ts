export class NetworkNode {
    id: string;
    label: string;
    posY: number;
    length: number;
  
    constructor(id: string, label: string) {
      this.id = id;
      this.label = label;

      this.posY = 0;
      this.length = 300;
    }
}