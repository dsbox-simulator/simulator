import React, {useMemo} from "react";
import {LogInfo, NodeInfo} from "../store/store";
import Tooltip from "./tooltip";
import {cssColor} from "../colors";

export default function LogView({nodes, logs, testNodeName}: {
    nodes: NodeInfo[],
    logs: LogInfo[],
    testNodeName: string
}) {
    const nodesById = useMemo(() => new Map<number, NodeInfo>(nodes.map(n => [n.id, n])), [nodes]);
    return <table className="table table-sm font-monospace">
        <thead>
        <tr>
            <th><Tooltip tooltip="sent at"><i className="bi bi-box-arrow-right"></i></Tooltip></th>
            <th>Node</th>
            <th>Text</th>
        </tr>
        </thead>
        <tbody>
        {logs.map((log: LogInfo) => <tr key={log.timestamp.logical}>
            <td>{log.timestamp.logical}</td>
            <td>{nodesById.get(log.node)?.name || <i>{testNodeName}</i>}</td>
            <td><span style={{color: cssColor(log.message.marker?.color || "Black")}}>{log.message.text}</span></td>
        </tr>)}
        </tbody>
    </table>;
}