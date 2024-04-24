import React from "react";
import {Log} from "../classes/Event";

const Logs: React.FC<{ logs: Map<number, Log[]> }> = ({logs}) => {
    const nodes = Array.from(logs.keys());
    return <>
        <nav>
            <div className="nav nav-tabs" id="nav-tab" role="tablist">
                {nodes.map(node => <button className="nav-link active" id={"nav-log-" + node} data-bs-toggle="tab" data-bs-target={"#nav-log-" + node}
                                           type="button" role="tab" aria-controls={"nav-log-" + node} aria-selected="true">{node}</button>)}
            </div>
        </nav>
        <div className="tab-content" id="nav-tabContent">
            {nodes.map(node => <div className="tab-pane fade show active" id={"nav-log-" + node} role="tabpanel" aria-labelledby={"nav-log-" + node}>
                logs for node {node}
            </div>)}
        </div>
    </>

}

export default Logs;