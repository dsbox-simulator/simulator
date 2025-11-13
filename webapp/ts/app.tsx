import React, {useEffect, useRef, useState} from "react";
import LamportDiagram from "./components/lamportDiagram";
import Store, {isLog, isMessage, LogInfo, MessageInfo} from "./store/store";
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
    const [commands, setCommands] = useState<Commands | null>(null);
    const [highlighted, setHighlighted] = useState<MessageInfo | LogInfo | null>(null);
    const setCommandsSave = (commands: Commands): void => {
        setCommands(commands);
        store.store("last_commands", commands);
    };

    useEffect(() => {
        (async () => {
            let store = storeRef.current!;
            let commands = await store.currentCommands()
            if (commands.testCommand.program === "") {
                let {testCommand, serverCommand} = await store.load("last_commands");
                setCommands({testCommand, serverCommand});
                store.restart(testCommand, serverCommand);
            } else {
                setCommandsSave(commands);
            }
        })();
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
                <LamportDiagram nodes={nodes}
                                messages={messages}
                                highlighted={highlighted}
                                setHighlighted={setHighlighted}
                                logs={logs}/>
            </div>
            <div className="messages-logs">
                <MessageView messages={messages}
                             filterNodes={new Set(nodes.map(n => n.name))}
                             highlighted={isMessage(highlighted) ? highlighted : null}
                             setHighlighted={setHighlighted}
                             onDeliver={m => store.deliver(m)}
                             onDrop={m => store.drop(m)}/>
                <LogView
                    nodes={nodes}
                    logs={logs}
                    highlighted={isLog(highlighted) ? highlighted : null}
                    setHighlighted={setHighlighted}/>
            </div>
        </div>
    </div>
}