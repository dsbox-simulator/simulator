import { GraphNode } from "./GraphNode";

export class GraphEdge {
    source: GraphNode;
    target: GraphNode;
    id: string;
    logicalTimestamp: number;
    label: string;

    constructor(source: GraphNode, target: GraphNode, id: string, logicalTimestamp: number, label: string) {
        this.source = source;
        this.target = target;
        this.id = id;
        this.logicalTimestamp = logicalTimestamp;
        this.label = label;
    }
}