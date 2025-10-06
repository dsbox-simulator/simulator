import React from "react"
import Tooltip from "./tooltip";

export default function Toolbar({onRestart, onStep, onResume, onBreak, connected}: {
    onRestart: () => void;
    onStep: () => void,
    onResume: () => void,
    onBreak: () => void,
    connected: boolean
}) {
    return <div className="d-flex align-items-center bg-body-tertiary border-bottom p-3 gap-3">
        <div>
            <Tooltip tooltip={connected ? "connected" : "disconnected"}>
                {connected ?
                    <i className="bi bi-circle-fill text-success"></i> :
                    <i className="bi bi-circle-fill text-danger"></i>
                }
            </Tooltip>
        </div>
        <div>
            <div className="btn-group btn-group-sm">
                <button onClick={onRestart} className="btn btn-outline-success"><i className="bi bi-arrow-counterclockwise"></i> Restart
                </button>
                <button onClick={onStep} className="btn btn-outline-success"><i className="bi bi-play"></i> Step
                </button>
                <button onClick={onBreak} className="d-none btn btn-outline-danger"><i className="bi bi-stop"></i> Break</button>
                <button onClick={onResume} className="btn btn-outline-success"><i
                    className="bi bi-fast-forward"></i> Resume
                </button>
            </div>
        </div>
    </div>
}