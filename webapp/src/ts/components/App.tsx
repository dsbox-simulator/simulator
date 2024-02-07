import React, {useEffect, useRef, useState} from "react";
import EventsTable from "./EventsTable";
import State from "../classes/State";
import CoreSocket from "../classes/CoreSocket";
import Controls from "./Controls";
import Nodes from "./Nodes";
import Logs from "./Logs";


const App: React.FC = () => {
    const [state, setState] = useState(new State());
    const socket = useRef<CoreSocket | null>(null);
    useEffect(() => {
        socket.current = new CoreSocket();
        socket.current.onevent = event => {
            setState(state => state.update(event))
        };
    }, []);

    return <div className="container">
        <Controls socket={socket.current!}/>
        <h2>Nodes</h2>
        <Nodes nodes={state.nodes}/>
        <h2>Events</h2>
        <EventsTable events={state.all_events}/>
        <h2>Logs</h2>
        <Logs logs={state.logs} />
    </div>;
}

export default App;