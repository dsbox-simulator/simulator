import React, {JSX, useMemo, useRef} from "react";
import * as bootstrap from "bootstrap";
import {createPortal} from "react-dom";

export default function Popover({children, tooltip}: { children: JSX.Element, tooltip: React.ReactNode }) {
    const contentElement = useMemo(() => document.createElement("div"), []);
    const ppref = useRef<bootstrap.Popover | null>(null);
    const enableTooltip = (element: HTMLElement | null) => {
        if (element === null) {
            if (ppref.current !== null) ppref.current.dispose();
            return;
        }
        ppref.current = new bootstrap.Popover(element, {
            html: true,
            content: contentElement,
            trigger: "click hover",
        });
    };
    return <>
        {React.cloneElement(children, {ref: enableTooltip})}
        {createPortal(tooltip, contentElement)}
    </>;
}