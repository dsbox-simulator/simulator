import React, {useEffect, useState} from "react"
import {open} from "@tauri-apps/plugin-dialog";
import Tooltip from "./tooltip";
import {Commands, displayCommand, splitCommand} from "../api/types";
import Modal from "./modal";
import {invoke} from "@tauri-apps/api/core";
import FileDropZone from "./fileDropZone";


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
    const [editTestCommand, setEditTestCommand] = useState<string>("");
    const [editServerCommand, setEditServerCommand] = useState<string>("");
    useEffect(() => {
        if (commands !== null) {
            setEditTestCommand(displayCommand(commands.testCommand));
            setEditServerCommand(displayCommand(commands.serverCommand));
        }
    }, [commands]);

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
                                <button className="btn btn-sm btn-primary" data-bs-toggle="modal"
                                        data-bs-target="#edit-commands">
                                    <i className="bi bi-pencil-square"></i>
                                </button>
                                <Modal className="modal-xl" id="edit-commands" title={'Edit nodes'}
                                       confirmButton={"Save & Restart"}
                                       onConfirm={() => {
                                           onRestart({
                                               testCommand: splitCommand(editTestCommand),
                                               serverCommand: splitCommand(editServerCommand),
                                           });
                                       }}>
                                    <EditCommands testCommand={editTestCommand} setTestCommand={setEditTestCommand}
                                                  serverCommand={editServerCommand}
                                                  setServerCommand={setEditServerCommand}/>
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

function EditCommands({testCommand, setTestCommand, serverCommand, setServerCommand}: {
    testCommand: string,
    setTestCommand: (cmd: string) => void,
    serverCommand: string,
    setServerCommand: (cmd: string) => void
}) {
    const [testPreferBuiltinLua, setTestPreferBuiltinLua] = useState<boolean>(true);
    const [serverPreferBuiltinLua, setServerPreferBuiltinLua] = useState<boolean>(true);

    const browseFor = async (setCommand: (cmd: string) => void, preferBuiltinLua: boolean) => {
        const file = await open({multiple: false, directory: false, defaultPath: "."});
        if (file === null) return;
        await setCommandFromPath(setCommand, preferBuiltinLua, file);
    }

    const setCommandFromPath = async (setCommand: (cmd: string) => void, preferBuiltinLua: boolean, path: string) => {
        const found = await invoke<{ language: string, interpreter: string } | null>("find_interpreter", {path});
        if (found === null || (found.language === "lua" && preferBuiltinLua)) {
            setCommand(path);
        } else {
            setCommand(`${found.interpreter} ${path}`);
        }
    }

    return <div>
        <div className="mb-3">
            <div className="d-flex justify-content-between">
                <label htmlFor="testComand">Test</label>
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
                <button className="btn btn-outline-secondary"
                        onClick={_ => browseFor(setTestCommand, testPreferBuiltinLua)}>Browse
                </button>
                <FileDropZone onDrop={paths => setCommandFromPath(setTestCommand, testPreferBuiltinLua, paths[0]!)}>
                    <input type="text" id="testComand" className="form-control"
                           value={testCommand}
                           onChange={e => setTestCommand(e.target.value)}/>
                </FileDropZone>
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
                <button className="btn btn-outline-secondary"
                        onClick={_ => browseFor(setServerCommand, serverPreferBuiltinLua)}>Browse
                </button>
                <FileDropZone onDrop={paths => setCommandFromPath(setServerCommand, serverPreferBuiltinLua, paths[0]!)}>
                    <input type="text" id="serverComand" className="form-control"
                           value={serverCommand}
                           onChange={e => setServerCommand(e.target.value)}/>
                </FileDropZone>
            </div>
        </div>
    </div>;
}