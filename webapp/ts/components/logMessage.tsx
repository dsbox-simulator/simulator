import {LogMessage} from "../api/types";
import {cssColor} from "../colors";
import React from "react";

export default function LogMessage({log}: { log: LogMessage }) {
    return <span className="font-monospace"
                 style={{color: cssColor(log.marker?.color || "Black")}}>{log.text}</span>;
}