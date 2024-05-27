import { NetworkNode } from "./NetworkNode";

export class GraphNode {
    id: string;
    label: string;
    networkNode: NetworkNode;
    posX: number;
  
    constructor(id: string, label: string, networkNode: NetworkNode) {
      this.id = id;
      this.label = label;
      this.networkNode = networkNode;

      this.posX = 0;
    }

    public get posY(): number {
      return this.networkNode.posY;
    }
}