import { GraphNode } from "./GraphNode";

export class GraphEdge {
    source: GraphNode;
    target: GraphNode;
    label: string;
    logicalTimestamp: number;

    constructor(source: GraphNode, target: GraphNode, label: string, logicalTimestamp: number) {
        this.source = source;
        this.target = target;
        this.label = label;
        this.logicalTimestamp = logicalTimestamp;
    }
}