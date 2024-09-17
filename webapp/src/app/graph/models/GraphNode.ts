import { NetworkNode } from "./NetworkNode";

/**
 * Represents a node in the graph
 */
export class GraphNode {
    id: string;
    label: string;
    networkNode: NetworkNode;
    posX: number;
    color: string | undefined;
  
    constructor(id: string, label: string, networkNode: NetworkNode) {
      this.id = id;
      this.label = label;
      this.networkNode = networkNode;

      this.posX = 0;
    }

    public get posY(): number {

      if(this.color === undefined)
      {
        return this.networkNode.posY;
      }
      else{
        //If color is set its a marker node. Needs to be higher Up to fit better on the Diagram
        return this.networkNode.posY - 15;
      }
    }
}