import { NetworkNode } from "./NetworkNode";

export class GraphNode {
    id: string;
    label: string;
    networkNode: NetworkNode;
    posX: number;
    posY: number;
  
    constructor(id: string, label: string, networkNode: NetworkNode) {
      this.id = id;
      this.label = label;
      this.networkNode = networkNode;

      this.posX = 0;
      this.posY = networkNode.posY;
    }
}