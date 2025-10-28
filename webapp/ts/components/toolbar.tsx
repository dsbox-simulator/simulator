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
    const [editCommands, setEditCommands] = useState<Commands>({
        testCommand: {program: "", args: []},
        serverCommand: {program: "", args: []}
    });
    useEffect(() => {
        if (commands !== null) setEditCommands(commands)
    }, [commands]);

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
                                <Modal className="modal-xl" open={openSettings} title={'Edit nodes'}
                                       confirmButton={"Save & Restart"}
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
        if (file === null) return;
        await setCommandFromPath(commandName, file);
    }

    const setCommandFromPath = async (commandName: "testCommand" | "serverCommand", path: string) => {
        const preferBuiltin = (commandName == "testCommand" && testPreferBuiltinLua) || (commandName == "serverCommand" && serverPreferBuiltinLua);
        const found = await invoke<{ language: string, interpreter: string } | null>("find_interpreter", {path});
        console.log("found", found, "perferBuiltin", preferBuiltin);
        if (found === null || (found.language === "lua" && preferBuiltin)) {
            console.log("nointerpreter");
            setCommands({...commands, [commandName]: {program: path, args: []}});
        } else {
            setCommands({...commands, [commandName]: {program: found.interpreter, args: [path]}});
        }
    }
    const setCommandFromString = (commandName: "testCommand" | "serverCommand", command: string) => {
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
                <FileDropZone onDrop={paths => setCommandFromPath("testCommand", paths[0]!)}>
                    <input type="text" id="testComand" className="form-control"
                           value={displayCommand(commands.testCommand)}
                           onChange={e => setCommandFromString("testCommand", e.target.value)}/>
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
                <button className="btn btn-outline-secondary" onClick={_ => browseFor("serverCommand")}>Browse
                </button>
                <FileDropZone onDrop={paths => setCommandFromPath("serverCommand", paths[0]!)}>
                    <input type="text" id="serverComand" className="form-control"
                           value={displayCommand(commands.serverCommand)}
                           onChange={e => setCommandFromString("serverCommand", e.target.value)}/>
                </FileDropZone>
            </div>
        </div>
    </div>;
}