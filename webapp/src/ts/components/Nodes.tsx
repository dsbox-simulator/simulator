import React from "react";
import {NodeInfo} from "../classes/Event";

const Nodes: React.FC<{ nodes: NodeInfo[] }> = ({nodes}) => {
    return <ul id="nodes">
        {nodes.map(node => <li key={node.id}>{node.name}</li>)}
    </ul>
}

export default Nodes;