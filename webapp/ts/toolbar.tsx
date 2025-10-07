import React from "react"
import Tooltip from "./tooltip";
import {Commands} from "./api/types";
import {open} from "@tauri-apps/plugin-dialog";

export default function Toolbar({onRestart, onStep, onResume, onBreak, commands, onSetCommands, inTauri, connected}: {
    onRestart: () => void;
    onStep: () => void,
    onResume: () => void,
    onBreak: () => void,
    commands: Commands | null,
    onSetCommands: (commands: Commands) => void,
    inTauri: boolean,
    connected: boolean
}) {
    return <div className="d-flex align-items-baseline bg-body-tertiary border-bottom p-3 gap-3">
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
                <button onClick={onRestart} className="btn btn-outline-success"><i
                    className="bi bi-arrow-counterclockwise"></i> Restart
                </button>
                <button onClick={onStep} className="btn btn-outline-success"><i className="bi bi-play"></i> Step
                </button>
                <button onClick={onBreak} className="d-none btn btn-outline-danger"><i className="bi bi-stop"></i> Break
                </button>
                <button onClick={onResume} className="btn btn-outline-success"><i
                    className="bi bi-fast-forward"></i> Resume
                </button>
            </div>
        </div>
        <div className="d-flex gap-2 align-items-baseline flex-grow-1">
            <label htmlFor="testCommand">Test</label>
            <div className="input-group input-group-sm">
                {inTauri &&
                    <button className="btn btn-outline-secondary" type="button" onClick={_ => open({
                        multiple: false,
                        directory: false,
                        defaultPath: ".",
                    }).then(file => {
                        if (file !== null) {
                            onSetCommands({
                                testCommand: file,
                                serverCommand: commands?.serverCommand || ""
                            });
                        }
                    })}>Browse</button>
                }
                <input type="text" className="form-control" value={commands?.testCommand || ""}
                       onChange={e => onSetCommands({
                           testCommand: e.target.value,
                           serverCommand: commands?.serverCommand || ""
                       })} id="testCommand" disabled={!inTauri}/>
            </div>
        </div>
        <div className="d-flex gap-2 align-items-baseline flex-grow-1">
            <label htmlFor="serverCommand">Servers</label>
            <div className="input-group input-group-sm">
                {inTauri &&
                    <button className="btn btn-outline-secondary" type="button" onClick={_ => open({
                        multiple: false,
                        directory: false,
                        defaultPath: ".",
                    }).then(file => {
                        if (file !== null) {
                            onSetCommands({
                                testCommand: commands?.testCommand || null,
                                serverCommand: file
                            });
                        }
                    })}>Browse</button>
                }
                <input type="text" className="form-control" value={commands?.serverCommand || ""}
                       onChange={e => onSetCommands({
                           serverCommand: e.target.value,
                           testCommand: commands?.testCommand || null
                       })} id="serverCommand" disabled={!inTauri}/>
            </div>
        </div>
    </div>
}