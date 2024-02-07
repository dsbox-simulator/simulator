import React from "react";
import CoreSocket from "../classes/CoreSocket";

const Controls: React.FC<{ socket: CoreSocket }> = ({socket}) => {
    return <>
        <button className="btn btn-primary" onClick={() => socket.send("step")}>Step</button>
        <button className="btn btn-primary" onClick={() => socket.send("resume")}>Resume</button>
    </>;
}

export default Controls;