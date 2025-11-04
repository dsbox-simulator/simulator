import React, {useMemo} from "react";
import {LogInfo, NodeInfo} from "../store/store";
import Tooltip from "./tooltip";
import LogMessage from "./logMessage";
import classNames from "classnames";

export default function LogView({nodes, logs, highlighted, setHighlighted, testNodeName}: {
    nodes: NodeInfo[],
    logs: LogInfo[],
    highlighted: LogInfo | null,
    setHighlighted: (log: LogInfo | null) => void,
    testNodeName: string
}) {
    const nodesById = useMemo(() => new Map<number, NodeInfo>(nodes.map(n => [n.id, n])), [nodes]);
    return <table className="table table-hover table-sm font-monospace">
        <thead>
        <tr>
            <th><Tooltip tooltip="sent at"><i className="bi bi-box-arrow-right"></i></Tooltip></th>
            <th>Node</th>
            <th>Text</th>
        </tr>
        </thead>
        <tbody>
        {logs.map((log: LogInfo) => <LogRow key={log.timestamp.logical}
                                            log={log}
                                            highlighted={highlighted}
                                            setHighLighted={setHighlighted}
                                            nodesById={nodesById}
                                            testNodeName={testNodeName}/>)}
        </tbody>
    </table>;
}

function LogRow({log, highlighted, setHighLighted, nodesById, testNodeName}: {
    log: LogInfo,
    highlighted: LogInfo | null,
    setHighLighted: (log: LogInfo | null) => void,
    nodesById: Map<number, NodeInfo>,
    testNodeName: string
}) {
    return <tr className={classNames({"table-secondary": log.timestamp.logical === highlighted?.timestamp.logical})}
               onMouseEnter={() => setHighLighted(log)} onMouseLeave={() => setHighLighted(null)}>
        <td>{log.timestamp.logical}</td>
        <td>{nodesById.get(log.node)?.name || <i>{testNodeName}</i>}</td>
        <td className="w-100"><LogMessage log={log.message}/></td>
    </tr>
}