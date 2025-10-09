import React, {useEffect, useState} from "react"
import {open} from "@tauri-apps/plugin-dialog";
import Tooltip from "./tooltip";
import {Command, Commands, displayCommand, splitCommand} from "../api/types";
import Modal from "./modal";
import {invoke} from "@tauri-apps/api/core";


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

export default function Toolbar({
                                    icon,
                                    onRestart,
                                    onStep,
                                    onResume,
                                    onBreak,
                                    commands,
                                    inTauri,
                                    connected
                                }: {
    icon: string,
    onRestart: (overrideCommands?: Commands) => void,
    onStep: () => void,
    onResume: () => void,
    onBreak: () => void,
    commands: Commands | null,
    inTauri: boolean,
    connected: boolean
}) {
    const [editCommands, setEditCommands] = useState<Commands>({
        testCommand: {program: "", args: []},
        serverCommand: {program: "", args: []}
    });
    useEffect(() => {if (commands !== null) setEditCommands(commands)}, [commands]);

    const [openSettings, setOpenSettings] = useState(false);

    return <nav className="navbar navbar-expand bg-body-tertiary border-bottom">
        <div className="container-fluid">
            <a className="navbar-brand">
                <img src={icon} alt="Icon" height="30"/>
            </a>
            <div className="navbar-collapse">
                <ul className="navbar-nav gap-2 align-items-center flex-grow-1">
                    {!inTauri &&
                        <li className="nav-item">
                            <Tooltip tooltip={connected ? "connected" : "disconnected"}>
                                {connected ?
                                    <i className="bi bi-circle-fill text-success"></i> :
                                    <i className="bi bi-circle-fill text-danger"></i>
                                }
                            </Tooltip>
                        </li>}
                    <li className="nav-item">
                        <div className="btn-group btn-group-sm">
                            <button onClick={() => onRestart()} className="btn btn-outline-success"><i
                                className="bi bi-arrow-counterclockwise"></i> Restart
                            </button>
                            <button onClick={onStep} className="btn btn-outline-success"><i
                                className="bi bi-play"></i> Step
                            </button>
                            <button onClick={onBreak} className="d-none btn btn-outline-danger"><i
                                className="bi bi-stop"></i> Break
                            </button>
                            <button onClick={onResume} className="btn btn-outline-success"><i
                                className="bi bi-fast-forward"></i> Resume
                            </button>
                        </div>
                    </li>
                    <li className="nav-item d-flex flex-grow-1 gap-2 align-items-center">
                        <div className="input-group input-group-sm">
                            {inTauri && <>
                                <button className="btn btn-sm btn-primary" onClick={_ => setOpenSettings(true)}>
                                    <i className="bi bi-pencil-square"></i>
                                </button>
                                <Modal open={openSettings} title={'Edit nodes'} confirmButton={"Save & Restart"}
                                       onClose={(confirmed) => {
                                           if (confirmed) {
                                               onRestart(editCommands);
                                           }
                                           setOpenSettings(false);
                                       }}>
                                    <EditNodes commands={editCommands} setCommands={setEditCommands}></EditNodes>
                                </Modal>
                            </>}
                            <span className="input-group-text">Test:</span>
                            <input type="text" className="form-control" disabled readOnly
                                   value={displayCommand(commands?.testCommand)}/>
                            <span className="input-group-text">Server:</span>
                            <input type="text" className="form-control" disabled readOnly
                                   value={displayCommand(commands?.serverCommand)}/>
                        </div>
                    </li>
                </ul>
            </div>
        </div>
    </nav>
}

function EditNodes({commands, setCommands}: { commands: Commands, setCommands: (commands: Commands) => void }) {
    const [testPreferBuiltinLua, setTestPreferBuiltinLua] = useState<boolean>(true);
    const [serverPreferBuiltinLua, setServerPreferBuiltinLua] = useState<boolean>(true);

    const browseFor = async (commandName: "testCommand" | "serverCommand") => {
        const file = await open({multiple: false, directory: false, defaultPath: "."});
        const preferBuiltin = (commandName == "testCommand" && testPreferBuiltinLua) || (commandName == "serverCommand" && serverPreferBuiltinLua);
        if (file === null) return;
        const found = await invoke<{ language: string, interpreter: string } | null>("find_interpreter", {file});
        console.log("found", found, "perferBuiltin", preferBuiltin);
        if (found === null || (found.language === "lua" && preferBuiltin)) {
            console.log("nointerpreter");
            setCommands({...commands, [commandName]: {program: file, args: []}});
        } else {
            setCommands({...commands, [commandName]: {program: found.interpreter, args: [file]}});
        }
    }

    const setCommand = (commandName: "testCommand" | "serverCommand", command: string) => {
        setCommands({...commands, [commandName]: splitCommand(command)});
    }

    return <div>
        <div className="mb-3">
            <div className="d-flex justify-content-between">
                <label htmlFor="testComand">Server</label>
                <Tooltip
                    tooltip={"When opening a `.lua` file prefer using the built-in interpreter instead of searching for a `lua` interpreter on the system"}>
                    <div className="form-check form-switch">
                        <input className="form-check-input" type="checkbox" role="switch" id="testPreferBuiltinLua"
                               checked={testPreferBuiltinLua}
                               onChange={e => setTestPreferBuiltinLua(e.target.checked)}/>
                        <label className="form-check-label" htmlFor="testPreferBuiltinLua">Prefer Built-in lua
                            interpreter</label>
                    </div>
                </Tooltip>
            </div>
            <div className="input-group">
                <button className="btn btn-outline-secondary" onClick={_ => browseFor("testCommand")}>Browse</button>
                <input type="text" id="testComand" className="form-control"
                       value={displayCommand(commands.testCommand)}
                       onChange={e => setCommand("testCommand", e.target.value)}/>
            </div>
        </div>
        <div className="mb-3">
            <div className="d-flex justify-content-between">
                <label htmlFor="serverComand">Server</label>
                <Tooltip
                    tooltip={"When opening a `.lua` file prefer using the built-in interpreter instead of searching for a `lua` interpreter on the system"}>
                    <div className="form-check form-switch">
                        <input className="form-check-input" type="checkbox" role="switch" id="serverPreferBuiltinLua"
                               checked={serverPreferBuiltinLua}
                               onChange={e => setServerPreferBuiltinLua(e.target.checked)}/>
                        <label className="form-check-label" htmlFor="serverPreferBuiltinLua">Prefer Built-in lua
                            interpreter</label>
                    </div>
                </Tooltip>
            </div>
            <div className="input-group">
                <button className="btn btn-outline-secondary" onClick={_ => browseFor("serverCommand")}>Browse
                </button>
                <input type="text" id="serverComand" className="form-control"
                       value={displayCommand(commands.serverCommand)}
                       onChange={e => setCommand("serverCommand", e.target.value)}/>
            </div>
        </div>
    </div>;
}