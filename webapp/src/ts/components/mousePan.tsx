import React, {JSX} from "react";

export default function MousePan({children}: { children: JSX.Element }) {
    const existingRef = children.props['ref'];
    const enablePan = (element: HTMLElement | null) => {
        if (element === null) return;
        element.style.setProperty("cursor", "grab");
        let state = {
            dragging: false,
            left: 0,
            top: 0,
            x: 0,
            y: 0,
        }
        const startDrag = (e: MouseEvent) => {
            element.style.setProperty("cursor", "grabbing");
            state.dragging = true;
            state.x = e.clientX;
            state.y = e.clientY;
            state.left = element.scrollLeft;
            state.top = element.scrollTop;
        }
        const stopDrag = (_e: MouseEvent) => {
            element.style.setProperty("cursor", "grab");
            element.style.removeProperty("pointer-events");
            state.dragging = false;
        }

        const drag = (e: MouseEvent) => {
            if (!state.dragging) return;
            const deltaX = e.clientX - state.x;
            const deltaY = e.clientY - state.y;
            element.scrollLeft = state.left - deltaX;
            element.scrollTop = state.top - deltaY;
        }
        element.addEventListener("mousedown", startDrag);
        document.addEventListener("mouseup", stopDrag);
        document.addEventListener("mouseleave", stopDrag);
        element.addEventListener("mousemove", drag);
        if (existingRef) existingRef(element);
    };

    return React.cloneElement(children, {ref: enablePan})
}