import { useEffect, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type FileDropState = {
    hovering: boolean;
    files: string[];
};

// Tauri v2: file drop events are delivered via @tauri-apps/api/event.
// Event payload may be an array of paths or an object with a `paths` field.
function extractPaths(payload: unknown): string[] {
    if (Array.isArray(payload) && payload.every((p) => typeof p === "string")) {
        return payload as string[];
    }
    if (
        payload &&
        typeof payload === "object" &&
        "paths" in (payload as Record<string, unknown>) &&
        Array.isArray((payload as { paths: unknown }).paths)
    ) {
        const paths = (payload as { paths: unknown[] }).paths;
        return paths.filter((p): p is string => typeof p === "string");
    }
    return [];
}

export function useTauriFileDrop(onDrop: (paths: string[], position: {x: number, y: number}) => void) {
    const [state, setState] = useState<FileDropState>({ hovering: false, files: [] });

    useEffect(() => {
        let unlistenFns: UnlistenFn[] = [];

        async function setup() {
            // Only attach listeners inside a Tauri WebView environment
            if (!(globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
                return;
            }
            const win = getCurrentWindow();

            // Global listeners (built-in Tauri v2 labels)
            unlistenFns.push(
                await listen("tauri://drag-enter", (e) => {
                    const paths = extractPaths(e.payload);
                    setState((prev) => ({ ...prev, hovering: true, files: paths.length ? paths : prev.files }));
                })
            );
            unlistenFns.push(
                await listen("tauri://drag-over", () => {
                    setState((prev) => ({ ...prev, hovering: true }));
                })
            );
            unlistenFns.push(
                await listen("tauri://drag-drop", (e) => {
                    const paths = extractPaths(e.payload);
                    setState({ hovering: false, files: paths });
                })
            );
            unlistenFns.push(
                await listen("tauri://drag-leave", () => {
                    setState((prev) => ({ ...prev, hovering: false }));
                })
            );

            // Window-scoped listeners (preferred)
            unlistenFns.push(
                await win.listen("tauri://drag-enter", (e) => {
                    const paths = extractPaths(e.payload);
                    setState((prev) => ({ ...prev, hovering: true, files: paths.length ? paths : prev.files }));
                })
            );
            unlistenFns.push(
                await win.listen("tauri://drag-over", () => {
                    setState((prev) => ({ ...prev, hovering: true }));
                })
            );
            unlistenFns.push(
                await win.listen("tauri://drag-drop", (e) => {
                    const paths = extractPaths(e.payload);
                    onDrop(paths, (e.payload as any).position);
                    setState({ hovering: false, files: paths });
                })
            );
            unlistenFns.push(
                await win.listen("tauri://drag-leave", () => {
                    setState((prev) => ({ ...prev, hovering: false }));
                })
            );

            // Custom backend-emitted labels (fallback from Rust)
            unlistenFns.push(
                await win.listen("file-drop-hover", (e) => {
                    const paths = extractPaths(e.payload);
                    setState((prev) => ({ ...prev, hovering: true, files: paths.length ? paths : prev.files }));
                })
            );
            unlistenFns.push(
                await win.listen("file-drop", (e) => {
                    const paths = extractPaths(e.payload);
                    setState({ hovering: false, files: paths });
                })
            );
            unlistenFns.push(
                await win.listen("file-drop-cancelled", () => {
                    setState((prev) => ({ ...prev, hovering: false }));
                })
            );
            // (Removed legacy v1-style labels)
        }

        setup();

        return () => {
            for (const u of unlistenFns) {
                try {
                    u();
                } catch (_) {
                    // ignore
                }
            }
        };
    }, [onDrop]);

    return state;
}

export type UseTauriFileDrop = ReturnType<typeof useTauriFileDrop>;
