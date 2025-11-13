import React, {useMemo} from "react";
import {LogInfo, NodeInfo} from "../store/store";
import Tooltip from "./tooltip";
import LogMessage from "./logMessage";
import classNames from "classnames";

export default function LogView(props: {
    nodes: NodeInfo[],
    logs: LogInfo[],
    highlighted: LogInfo | null,
    setHighlighted: (log: LogInfo | null) => void
}) {
    const [wrapLines, setWrapLines] = React.useState<boolean>(true);
    return <div className="tool-pane">
        <div className="tool-pane-header">
            <div><i className="bi bi-terminal"></i> Logs</div>
            <div className="form-check form-switch">
                <input className="form-check-input" type="checkbox" role="switch" id="wrapLines"
                       checked={wrapLines}
                       onChange={e => {
                           console.log(wrapLines, !e.target.checked);
                           setWrapLines(e.target.checked);
                       }}/>
                <label className="form-check-label" htmlFor="wrapLines">Wrap lines</label>
            </div>
        </div>
        <div className="tool-pane-content overflow-y-scroll">
            <LogTable wrapLines={wrapLines} {...props}/>
        </div>
    </div>;
}

function LogTable({nodes, logs, ...props}: {
    nodes: NodeInfo[],
    logs: LogInfo[],
    highlighted: LogInfo | null,
    setHighlighted: (log: LogInfo | null) => void
    wrapLines: boolean
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
                                            nodesById={nodesById}
                                            {...props}/>)}
        </tbody>
    </table>;
}

function LogRow({log, highlighted, setHighlighted, nodesById, wrapLines}: {
    log: LogInfo,
    highlighted: LogInfo | null,
    setHighlighted: (log: LogInfo | null) => void,
    nodesById: Map<number, NodeInfo>,
    wrapLines: boolean
}) {
    return <tr className={classNames({"table-secondary": log.timestamp.logical === highlighted?.timestamp.logical})}
               onMouseEnter={() => setHighlighted(log)} onMouseLeave={() => setHighlighted(null)}>
        <td>{log.timestamp.logical}</td>
        <td>{nodesById.get(log.node)?.name}</td>
        <td className="w-100"><LogMessage log={log.message} wrapLines={wrapLines}/></td>
    </tr>
}