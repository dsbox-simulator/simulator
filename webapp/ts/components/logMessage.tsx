import {LogMessage} from "../api/types";
import {cssColor} from "../colors";
import React from "react";

export default function LogMessage({log}: { log: LogMessage }) {
    return <pre className="font-monospace mb-0"
                style={{color: cssColor(log.marker?.color || "Black")}}>{log.text}</pre>;
}