import React, {JSX, useRef} from "react";
import * as bootstrap from "bootstrap";

export default function Tooltip({children, tooltip}: { children: JSX.Element, tooltip: string }) {
    const existingRef = children.props['ref'];
    const ttref = useRef<bootstrap.Tooltip | null>(null);
    const enableTooltip = (element: HTMLElement | null) => {
        if (element === null) {
            if (ttref.current !== null) ttref.current.dispose();
            return;
        }
        ttref.current = new bootstrap.Tooltip(element, {title: tooltip, trigger: "hover"});
        if (existingRef) existingRef(element);
    };
    return React.cloneElement(children, {ref: enableTooltip});
}