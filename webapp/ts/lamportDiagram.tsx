import React, {useMemo, useState} from "react";
import {Circle, Point, Segment, Shape} from "@flatten-js/core";
import Popover from "./popover";
import {Json} from "./json";
import {LogInfo, MessageInfo, NodeInfo} from "./store/store";
import {cssColor} from "./colors";

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
}

interface LamportDiagramProps {
    nodes: string[],
    events: Event[],
    communications: Communication[],
    nodeSpacing?: number,
    defaultEventSpacing?: number,
    eventRadius?: number,
}

function selectColor(nmb: number): string {
    const hue = nmb * 137.508; // use golden angle approximation
    return `hsl(${hue},75%,50%)`;
}

function toLamportProps(nodes: NodeInfo[],
                        messages: MessageInfo[],
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
    for (const message of messages) {
        if (message.message.src === "core" || message.message.dest === "core") continue;
        const sender = nodesByName.get(message.message.src);
        if (sender === undefined) continue;
        events.push({
            node: sender,
            logicalClock: message.sentAt.logical,
            data: message.message.body,
            color: message.dropped ? "red" : "white",
        });
        if (message.deliveredAt !== null) {
            const receiver = nodesByName.get(message.message.dest);
            if (receiver === undefined) continue;
            events.push({
                node: receiver,
                logicalClock: message.deliveredAt.logical,
                data: message.message.body,
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
            });
        }
    }
    for (const log of logs) {
        const node = nodesById.get(log.node);
        if (node === undefined) continue;
        events.push({
            node,
            data: log.message.text,
            logicalClock: log.timestamp.logical,
            label: log.message.marker?.label,
            color: cssColor(log.message.marker?.color || "Black"),
        });
    }
    return [{
        nodes: nodeNames,
        events,
        communications,
    }, colorMap]
}

export default function LamportDiagram({nodes, messages, logs, testNodeName}: {
    nodes: NodeInfo[],
    messages: MessageInfo[],
    logs: LogInfo[]
    testNodeName: string,
}) {
    const [lamportProps, colorMap] = useMemo(() => toLamportProps(nodes, messages, logs, testNodeName), [nodes, messages, logs]);
    return <div>
        <LamportDiagramImpl {...lamportProps} />
        <Legend colors={colorMap}/>
    </div>;
}

function LamportDiagramImpl({
                                nodes,
                                events,
                                communications,
                                nodeSpacing = 75,
                                defaultEventSpacing = 50,
                                eventRadius = 6
                            }: LamportDiagramProps) {
    const [minWidth, setMinWidth] = useState(0);
    const [eventSpacing, setEventSpacing] = useState(defaultEventSpacing);
    const lastTimestamp = Math.max(0, ...events.map(e => e.logicalClock));
    const firstTimestamp = Math.min(lastTimestamp, ...events.map(e => e.logicalClock));
    const range = lastTimestamp - firstTimestamp;
    const width = Math.max((range + 1) * eventSpacing, minWidth);
    const height = nodeSpacing * nodes.length;

    const timelineY = (nodeIdx: number) => (nodeIdx + 0.5) * nodeSpacing;
    const timelinePos = (nodeIdx: number, logicalTimestamp: number) =>
        new Point(((logicalTimestamp - firstTimestamp) + 0.5) * eventSpacing, timelineY(nodeIdx));

    return <div className="d-flex overflow-x-scroll mb-1">
        <div className="flex-shrink-1 position-sticky bg-white border-end start-0">
            {nodes.map(n => <div key={n} className="d-flex align-items-center px-2"
                                 style={{"height": nodeSpacing}}>{n}</div>)}
        </div>
        <div ref={e => setMinWidth(e?.clientWidth || 0)} className="w-100">
            <svg width={width} height={Math.max(height, 300)}>
                <defs>
                    <Marker id="marker"/>
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
                    {communications.map(comm => {
                        const sentP = timelinePos(comm.from, comm.sentLogicalClock);
                        const receivedP = timelinePos(comm.to, comm.receivedLogicalClock);
                        const fullSegment = new Segment(sentP, receivedP);
                        // inset the line segment by the radius of the event markers + the end marker arrow
                        const segment = new Segment(
                            fullSegment.ps.translate(fullSegment.tangentInStart().multiply(eventRadius)),
                            fullSegment.pe.translate(fullSegment.tangentInEnd().multiply(eventRadius + 10)));
                        return <AsSvg key={comm.sentLogicalClock} shape={segment} markerEnd="url(#marker)"
                                      stroke={comm.color}/>
                    })}
                </g>
                <g stroke="currentColor" fill="white">
                    {events.map(event =>
                        <EventLabel key={event.logicalClock} event={event}
                                    pos={timelinePos(event.node, event.logicalClock)}
                                    radius={eventRadius}/>)}
                </g>
            </svg>
        </div>
    </div>;
}

function EventLabel({event, pos, radius}: { event: Event, pos: Point, radius: number }) {
    if (event.label === undefined) {
        return <Popover tooltip={<Json json={event.data} format={true}/>}>
            <AsSvg shape={new Circle(pos, radius)} fill={event.color}/>
        </Popover>;
    } else {
        return <foreignObject x={pos.x - 25} y={pos.y - 50} width="50" height="50">
            <div style={{position: "relative", width: 50, height: 50}}>
                <div className="d-inline-block" style={{position: "absolute", left: "50%", bottom: 0}}>
                    <Popover tooltip={<Json json={event.data} format={true}/>}>
                        <div className="lamport-label"
                             style={{"--label-color": event.color} as React.CSSProperties}>{event.label}</div>

                    </Popover>
                </div>
            </div>
        </foreignObject>
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

function Legend({colors}: { colors: Map<string, string> }) {
    return <div className="border-top sticky-bottom bg-white p-2">
        Message Types:
        {[...colors.entries()].map(([type, color]) => <div key={type} className="badge rounded-pill ms-2"
                                                           style={{background: color}}>{type}</div>)}
    </div>
}