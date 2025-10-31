import React, {useMemo, useRef, useState} from "react";
import {Circle, Point, Segment, Shape} from "@flatten-js/core";
import {Json} from "./json";
import {isLog, isMessage, LogInfo, MessageInfo, NodeInfo} from "../store/store";
import {cssColor} from "../colors";
import {createPortal} from "react-dom";
import MousePan from "./mousePan";
import classNames from "classnames";
import LogMessage from "./logMessage";

interface Event {
    node: number
    logicalClock: number,
    label?: string,
    color: string,
    data: any
}

interface Communication {
    from: number,
    to: number,
    sentLogicalClock: number,
    receivedLogicalClock: number,
    color: string,
    data: any,
}

interface LamportDiagramProps {
    nodes: string[],
    events: Event[],
    communications: Communication[],
    highlights: { event?: number, communication?: number }[],
    getHoverData?: (data: any | null) => React.ReactNode,
    defaultNodeSpacing?: number,
    defaultEventSpacing?: number,
    eventRadius?: number,
}

function selectColor(nmb: number): string {
    const hue = nmb * 137.508; // use golden angle approximation
    return `hsl(${hue},75%,50%)`;
}


function toLamportProps(nodes: NodeInfo[],
                        messages: MessageInfo[],
                        highlighted: MessageInfo | LogInfo | null,
                        logs: LogInfo[],
                        testNodeName: string): [LamportDiagramProps, Map<string, string>] {
    const nodeNames = [testNodeName, ...nodes.map(n => n.name)];
    const colorMap = new Map<string, string>();
    const nodesByName = new Map<string, number>(nodeNames.map((n, i) => [n, i]));
    const nodesById = new Map<number, number>(nodes.map((n, i) => [n.id, i + 1]));
    nodesByName.set(testNodeName, 0);
    nodesById.set(0, 0);
    const events: Event[] = [];
    const communications: Communication[] = [];
    const highlights: { event?: number, communication?: number }[] = [];
    for (const message of messages) {
        const isHighlighted = isMessage(highlighted) && message.sentAt === highlighted.sentAt;
        if (message.message.src === "core" || message.message.dest === "core") continue;
        const sender = nodesByName.get(message.message.src);
        if (sender === undefined) continue;
        events.push({
            node: sender,
            logicalClock: message.sentAt.logical,
            data: message,
            color: message.dropped ? "red" : "white",
        });
        if (message.deliveredAt !== null) {
            const receiver = nodesByName.get(message.message.dest);
            if (receiver === undefined) continue;
            events.push({
                node: receiver,
                logicalClock: message.deliveredAt.logical,
                data: message,
                color: "white",
            });
            let color = colorMap.get(message.message.body.type);
            if (color === undefined) {
                color = selectColor(colorMap.size);
                colorMap.set(message.message.body.type, color);
            }

            communications.push({
                from: sender,
                to: receiver,
                sentLogicalClock: message.sentAt.logical,
                receivedLogicalClock: message.deliveredAt.logical,
                color,
                data: message,
            });
            if (isHighlighted) {
                highlights.push({communication: communications.length - 1});
            }
        } else {
            if (isHighlighted) {
                highlights.push({event: events.length - 1});
            }
        }
    }
    for (const log of logs) {
        const isHighlighted = isLog(highlighted) && log.timestamp.logical == highlighted.timestamp.logical;
        const node = nodesById.get(log.node);
        if (node === undefined) continue;
        events.push({
            node,
            data: log,
            logicalClock: log.timestamp.logical,
            label: log.message.marker?.label,
            color: cssColor(log.message.marker?.color || "Black"),
        });
        if (isHighlighted) {
            highlights.push({event: events.length - 1});
        }
    }
    return [{
        nodes: nodeNames,
        events,
        communications,
        highlights,
    }, colorMap]
}

export default function LamportDiagram({nodes, messages, highlighted, setHighlighted, logs, testNodeName}: {
    nodes: NodeInfo[],
    messages: MessageInfo[],
    highlighted: MessageInfo | LogInfo | null,
    setHighlighted: (highlighted: MessageInfo | LogInfo | null) => void,
    logs: LogInfo[]
    testNodeName: string,
}) {
    const [lamportProps, colorMap] = useMemo(() =>
            toLamportProps(nodes, messages, highlighted, logs, testNodeName),
        [nodes, messages, highlighted, logs]);
    const onHover = (data: MessageInfo | LogInfo | null): React.ReactNode => {
        setHighlighted(data);
        if (isMessage(data)) {
            return <Json json={data.message.body}/>;
        } else if (isLog(data)) {
            return <LogMessage log={data.message}/>
        } else {
            return null;
        }
    }

    return <div className="h-100 position-relative">
        <LamportDiagramImpl {...lamportProps} getHoverData={onHover}/>
        <Legend colors={colorMap}/>
    </div>;
}

function LamportDiagramImpl({
                                nodes,
                                events,
                                communications,
                                highlights,
                                getHoverData,
                                defaultNodeSpacing = 75,
                                defaultEventSpacing = 50,
                                eventRadius = 6
                            }: LamportDiagramProps) {
    const [minWidth, setMinWidth] = useState(0);

    const [hoverContent, setHoverContent] = useState<React.ReactNode | null>(null);

    const [eventSpacing, setEventSpacing] = useState(defaultEventSpacing);
    const [nodeSpacing, setNodeSpacing] = useState(defaultNodeSpacing);

    const lastTimestamp = Math.max(0, ...events.map(e => e.logicalClock));
    const firstTimestamp = Math.min(lastTimestamp, ...events.map(e => e.logicalClock));
    const range = lastTimestamp - firstTimestamp;
    const topPadding = 8; // reserve some padding on top for the lamport timestamps
    const width = Math.max((range + 1) * eventSpacing, minWidth);
    // makes the diagram at least 300px high
    const height = Math.max(nodeSpacing * nodes.length + topPadding, 300);


    const timelineY = (nodeIdx: number) => topPadding + (nodeIdx + 0.5) * nodeSpacing;
    const timelinePos = (nodeIdx: number, logicalTimestamp: number) =>
        new Point(((logicalTimestamp - firstTimestamp) + 0.5) * eventSpacing, timelineY(nodeIdx));

    const hover = (data: any | null) => {
        if (getHoverData) {
            setHoverContent(getHoverData(data));
        }
        return true;
    }


    const enableZoom = (element: HTMLDivElement | null) => {
        if (element === null) return;
        element.addEventListener("wheel", event => {
            if (event.ctrlKey) {
                const spacing = eventSpacing - event.deltaY * 0.1;
                setEventSpacing(Math.min(Math.max(spacing, 24), 100));
                event.preventDefault();
                return false;
            } else if (event.altKey) {
                const spacing = nodeSpacing - event.deltaY * 0.1;
                setNodeSpacing(Math.min(Math.max(spacing, 24), 100));
                event.preventDefault();
                return false;
            }
            return true;
        }, {passive: false});
    };

    return <MousePan>
        <div className="d-flex overflow-scroll h-100 pb-5" ref={enableZoom}>
            {hoverContent !== null && createPortal(<FollowMouse>
                {hoverContent}
            </FollowMouse>, document.body)}
            <div className="flex-shrink-1 position-sticky bg-white border-end start-0" style={{height}}>
                {nodes.map(n => <div key={n} className="d-flex align-items-center px-2"
                                     style={{"height": nodeSpacing}}>{n}</div>)}
            </div>
            <div ref={e => setMinWidth(e?.clientWidth || 0)} className="flex-grow-1" style={{height}}>
                <svg width={width} height={height}>
                    <defs>
                        <Marker id="marker"/>
                        <Pin id="pin"/>
                    </defs>
                    <g fontSize={8}>
                        {[...Array(range)].map((_, i) => <text key={i}
                                                               x={timelinePos(0, i + firstTimestamp).x - eventRadius / 2}
                                                               y={10}>{i + firstTimestamp}</text>)}
                    </g>
                    <g stroke="currentColor">
                        {nodes.map((name, idx) => <AsSvg key={name}
                                                         shape={new Segment(new Point(0, timelineY(idx)), new Point(width, timelineY(idx)))}/>)}
                    </g>
                    <g stroke="currentColor">
                        {communications.map((comm, index) => {
                            const highlighted = highlights.find(h => h.communication === index) !== undefined;
                            const sentP = timelinePos(comm.from, comm.sentLogicalClock);
                            const receivedP = timelinePos(comm.to, comm.receivedLogicalClock);
                            const fullSegment = new Segment(sentP, receivedP);
                            // inset the arrow segment by the radius of the event markers + the end marker arrow
                            const arrowSegment = new Segment(
                                fullSegment.ps.translate(fullSegment.tangentInStart().multiply(eventRadius)),
                                fullSegment.pe.translate(fullSegment.tangentInEnd().multiply(eventRadius + 10)));
                            return <g className={classNames("lamport-comm", {highlighted})}
                                      key={comm.sentLogicalClock}
                                      onMouseEnter={() => hover(comm.data)}
                                      onMouseLeave={() => hover(null)}>
                                <AsSvg shape={fullSegment}
                                       className="lamport-comm-highlight"
                                       strokeWidth={eventRadius * 3}
                                       strokeLinecap="round"></AsSvg>
                                <AsSvg shape={arrowSegment} markerEnd="url(#marker)"
                                       stroke={comm.color}/>
                            </g>;
                        })}
                    </g>
                    <g stroke="currentColor" fill="white">
                        {events.map((event, index) =>
                            <EventLabel key={event.logicalClock} event={event}
                                        pos={timelinePos(event.node, event.logicalClock)}
                                        radius={eventRadius}
                                        highlighted={highlights.find(h => h.event === index) !== undefined}
                                        hover={hover}/>)}
                    </g>
                </svg>
            </div>
        </div>
    </MousePan>;
}

function EventLabel({event, pos, radius, highlighted, hover}: {
    event: Event,
    pos: Point,
    radius: number,
    highlighted: boolean,
    hover: (data: any | null) => boolean,
}) {
    if (event.label === undefined) {
        return <g className={classNames("lamport-event", {highlighted})}>
            <AsSvg className="lamport-event-highlight" shape={new Circle(pos, radius + 3)}/>
            <AsSvg shape={new Circle(pos, radius)} fill={event.color}
                   onMouseEnter={() => hover(event.data)}
                   onMouseLeave={() => hover(null)}/>
        </g>;
    } else {
        const fontSize = 12;
        const padding = 4;
        const margin = 3;
        const length = Math.min(event.label.length, 3);
        const label = event.label.substring(0, length);
        const innerWidth = length * fontSize + padding * 2;
        const innerHeight = fontSize + padding * 2;
        const outerWidth = innerWidth + margin * 2;
        const outerHeight = innerHeight + margin * 2;
        const x = pos.x - outerWidth / 2;
        const y = pos.y - outerHeight / 2;
        return <g className={classNames("lamport-event", {highlighted})}
                  transform={`translate(${x}, ${y})`}
                  stroke="none"
                  onMouseEnter={() => hover(event.data)}
                  onMouseLeave={() => hover(null)}>
            <rect className="lamport-event-highlight" width={innerWidth + margin * 2}
                  height={innerHeight + margin * 2} rx={radius}/>
            <rect x={margin} y={margin} width={innerWidth} height={innerHeight} fill={event.color} rx={radius}/>
            <text x={outerWidth / 2} y={outerHeight / 2} style={{textAnchor: "middle", dominantBaseline: "middle"}}
                  fontSize={fontSize} fill="black">{label}</text>
        </g>
    }
}

function AsSvg({shape, children, ...svgProps}: { shape: Shape, children?: React.ReactNode, [svgProps: string]: any }) {
    if (shape instanceof Circle) {
        return <circle cx={shape.center.x} cy={shape.center.y} r={shape.r} {...svgProps}>{children}</circle>
    } else if (shape instanceof Segment) {
        return <line x1={shape.ps.x} y1={shape.ps.y} x2={shape.pe.x} y2={shape.pe.y} {...svgProps}>{children}</line>
    } else {
        return null;
    }
}

function Marker({id}: { id: string }) {
    return <marker
        id={id}
        viewBox="0 0 10 10"
        refX="1"
        refY="5"
        markerUnits="strokeWidth"
        markerWidth="10"
        markerHeight="10"
        orient="auto">
        <path d="M 0 0 L 10 5 L 0 10 z"/>
    </marker>;
}

function Pin({id}: { id: string }) {
    return <symbol id={id} viewBox="0 0 12 16" refX="6" refY="0">
        <path d="
            m 6,16
            c 0,0 6,-5.686 6,-10
            A 6,6 0 0 0 0,6
            c 0,4.314 6,10 6,10
            M 6,9
            A 3,3 0 1 1 6,3 3,3 0 0 1 6,9"/>
    </symbol>
}

function Legend({colors}: { colors: Map<string, string> }) {
    return <div className="border-top position-absolute bottom-0 w-100 bg-white p-2">
        Message Types:
        {[...colors.entries()].map(([type, color]) => <div key={type} className="badge rounded-pill ms-2"
                                                           style={{background: color}}>{type}</div>)}
    </div>
}

function FollowMouse({children}: { children: React.ReactNode }) {
    const ttref = useRef<{ elem: HTMLDivElement | null, listener: any }>({elem: null, listener: null});
    const followMouse = (element: HTMLDivElement | null) => {
        if (element === null) {
            if (ttref.current.listener !== null) {
                document.removeEventListener("mousemove", ttref.current.listener);
            }
            return;
        }
        const listener = (e: MouseEvent) => {
            const left_center = e.clientX - element.clientWidth / 2;
            const left = Math.min(Math.max(left_center, 0), document.body.clientWidth - element.clientWidth);
            let top
            if (e.clientY < element.clientHeight + 10) {
                top = e.clientY + 10;
            } else {
                top = e.clientY - element.clientHeight - 10;
            }
            element.style.setProperty("left", left + "px");
            element.style.setProperty("top", top + "px");
        }
        document.addEventListener("mousemove", listener);
        ttref.current = {elem: element, listener: listener};
    }

    return <div ref={followMouse} className="card position-fixed" style={{fontSize: ".875rem", zIndex: 9999}}>
        <div className="card-body">{children}</div>
    </div>
}