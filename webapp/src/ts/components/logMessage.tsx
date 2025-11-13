import {LogMessage} from "../api/types";
import {cssColor} from "../colors";
import React from "react";

export default function LogMessage({log, wrapLines}: { log: LogMessage, wrapLines?: boolean }) {
    wrapLines = wrapLines || false;
    return <pre className="font-monospace mb-0"
                style={{
                    color: cssColor(log.marker?.color || "Black"),
                    whiteSpace: wrapLines ? "pre-wrap" : undefined
                }}>{log.text}</pre>;
}