import React from "react"
import Tooltip from "./tooltip";
import {Command, Commands, displayCommand, splitCommand} from "./api/types";
import {open} from "@tauri-apps/plugin-dialog";

const INTERPRETERS_BY_EXTENSION: { [extension: string]: string } = {
    ".py": "python",
    ".js": "node",
    ".pl": "perl",
    ".rb": "ruby",
};

function commandFrom(commandStr: string, split: boolean): Command {
    let command: Command;
    if (split) {
        command = splitCommand(commandStr);
    } else {
        command = {program: commandStr, args: []};
    }

    for (const extension of Object.keys(INTERPRETERS_BY_EXTENSION)) {
        if (command.program.endsWith(extension)) {
            command.args.splice(0, 0, command.program);
            command.program = INTERPRETERS_BY_EXTENSION[extension]!;
        }
    }
    return command;
}

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
    const setCommands = ({testCommand, serverCommand}: {
        testCommand?: string,
        serverCommand?: string
    }, split: boolean) => {
        onSetCommands({
            testCommand: testCommand !== undefined ? commandFrom(testCommand, split) : (commands?.testCommand || {
                program: "",
                args: []
            }),
            serverCommand: serverCommand !== undefined ? commandFrom(serverCommand, split) : (commands?.serverCommand || {
                program: "",
                args: []
            })
        })
    }

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
                        if (file !== null) setCommands({testCommand: file}, false);
                    })}>Browse</button>
                }
                <input type="text" className="form-control"
                       value={displayCommand(commands?.testCommand || {program: "", args: []})}
                       onChange={e => setCommands({testCommand: e.target.value}, true)}
                       id="testCommand" disabled={!inTauri}/>
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
                        if (file !== null) setCommands({serverCommand: file}, false);
                    })}>Browse</button>
                }
                <input type="text" className="form-control"
                       value={displayCommand(commands?.serverCommand || {program: "", args: []})}
                       onChange={e => setCommands({serverCommand: e.target.value}, true)}
                       id="serverCommand" disabled={!inTauri}/>
            </div>
        </div>
    </div>
}