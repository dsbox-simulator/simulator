import React, {useEffect, useRef, useState} from "react";
import LamportDiagram from "./components/lamportDiagram";
import Store from "./store/store";
import Toolbar from "./components/toolbar";
import MessageView from "./components/messageView";
import LogView from "./components/logView";
import {Commands} from "./api/types";

// @ts-ignore
import icon from "../res/icon.png";

export default function App({wsPath, inTauri}: { wsPath: string, inTauri: boolean }) {
    const storeRef = useRef<Store | null>(null);

    if (storeRef.current === null) {
        storeRef.current = new Store(wsPath, inTauri);
    }
    const store = storeRef.current;
    const nodes = store.useNodes();
    const connected = store.useConnected();
    const logs = store.useLogs();
    const messages = store.useMessages();
    const testNodeName = "test";
    const [showOnlyUndelivered, setShowOnlyUndelivered] = useState(true);
    const [commands, setCommands] = useState<Commands | null>(null);
    const setCommandsSave = (commands: Commands): void => {
        setCommands(commands);
        storeRef.current!.store("last_commands", commands);
    };

    useEffect(() => {
        storeRef.current!.currentCommands().then(commands => {
            if (commands.testCommand.program === "") {
                storeRef.current!.load("last_commands")
                    .then(commands => setCommands(commands));
            } else {
                setCommandsSave(commands)
            }
        })

    }, []);

    return <div id="main">
        <div className="toolbar">
            <Toolbar
                icon={icon}
                onRestart={(overrideCommands?: Commands) => {
                    let useCommands = commands;
                    if (overrideCommands !== undefined) {
                        setCommandsSave(overrideCommands);
                        useCommands = overrideCommands;
                    }
                    store.restart(useCommands?.testCommand || undefined, useCommands?.serverCommand)
                }}
                onStep={() => store.step()}
                onResume={() => store.resume()}
                onBreak={() => store.break()}
                commands={commands}
                inTauri={inTauri}
                connected={connected}/>
        </div>
        <div className="content">
            <div className="lamport-diagram">
                <LamportDiagram nodes={nodes} messages={messages} logs={logs} testNodeName={testNodeName}/>
            </div>
            <div className="messages-logs">
                <div className="tool-pane">
                    <div className="tool-pane-header">
                        <div>
                            <i className="bi bi-envelope"></i> Messages
                        </div>
                        <div className="form-check form-switch">
                            <input className="form-check-input" type="checkbox" role="switch" id="showAllMessages"
                                   checked={!showOnlyUndelivered}
                                   onChange={e => setShowOnlyUndelivered(!e.target.checked)}/>
                            <label className="form-check-label" htmlFor="showAllMessages">Show all messages</label>
                        </div>
                    </div>
                    <div className="tool-pane-content overflow-y-scroll">
                        <MessageView messages={messages} onlyUndelivered={showOnlyUndelivered}
                                     onDeliver={m => store.deliver(m)}
                                     onDrop={m => store.drop(m)}/>
                    </div>
                </div>
                <div className="tool-pane">
                    <div className="tool-pane-header">
                        <div><i className="bi bi-terminal"></i> Logs</div>
                    </div>
                    <div className="tool-pane-content overflow-y-scroll">
                        <LogView nodes={nodes} logs={logs} testNodeName={testNodeName}/>
                    </div>
                </div>
            </div>
        </div>
    </div>
}