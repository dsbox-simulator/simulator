import React from "react";

const Nodes: React.FC<{ nodes: Map<string, number> }> = ({nodes}) => {
    return <ul id="nodes">
        {Array.from(nodes.keys()).map(node => <li key={node}>{node}</li>)}
    </ul>
}

export default Nodes;