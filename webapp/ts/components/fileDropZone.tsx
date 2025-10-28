import React from "react";
import {useTauriFileDrop} from "../hooks/useTauriFileDrop";


export default function FileDropZone({onDrop, children}: {
    onDrop: (paths: string[]) => void,
    children: React.JSX.Element
}) {
    useTauriFileDrop((paths, position) => {
        if (ref.current === null) return;
        console.log(paths, position);
        const target = ref.current.getBoundingClientRect();
        if (target.left <= position.x && target.right >= position.x && target.top <= position.y && target.bottom >= position.y) {
            onDrop(paths)
        }
    });
    const ref = React.useRef<HTMLElement | null>(null);
    const existingRef = children.props['ref'];
    const captureHoverState = (element: HTMLElement | null) => {
        if (existingRef) existingRef(element);
        ref.current = element;
    };
    return React.cloneElement(children, {ref: captureHoverState});
}