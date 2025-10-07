import React from "react";
import {MessageInfo} from "./store/store";
import Tooltip from "./tooltip";
import {Json} from "./json";

function messageFilter(message: MessageInfo, onlyUndelivered: boolean): boolean {
    if (onlyUndelivered && message.deliveredAt !== null) return false;
    return !(message.message.src === "core" || message.message.dest === "core");
}

export default function MessageView({messages, onlyUndelivered, onDeliver, onDrop}: {
    messages: MessageInfo[],
    onlyUndelivered: boolean
    onDeliver: (messages: MessageInfo) => void;
    onDrop: (messages: MessageInfo) => void;
}) {
    const shownMessages = messages.filter(m => messageFilter(m, onlyUndelivered));
    return <table className="table table-small font-monospace">
        <thead>
        <tr className="sticky-top">
            <th><Tooltip tooltip="sent at"><i className="bi bi-box-arrow-right"></i></Tooltip></th>
            {!onlyUndelivered &&
                <th><Tooltip tooltip="delivered at"><i className="bi bi-box-arrow-in-right"></i></Tooltip></th>}
            <th>From</th>
            <th>To</th>
            <th>Type</th>
            <th>Content</th>
            <th></th>
        </tr>
        </thead>
        <tbody>
        {shownMessages.map(message =>
            <tr key={message.sentAt.logical} className={message.dropped ? "d-none" : undefined}>
                <td>{message.sentAt.logical}</td>
                {!onlyUndelivered && <td>{message.deliveredAt?.logical}</td>}
                <td>{message.message.src}</td>
                <td>{message.message.dest}</td>
                <td>{message.message.body.type}</td>
                <td><Json json={{...message.message.body, type: undefined}} format={false}/></td>
                <td>
                    {message.deliveredAt === null &&
                        <div className="btn-group">
                            <Tooltip tooltip="deliver now">
                                <button className="btn btn-sm btn-outline-success"
                                        onClick={() => onDeliver(message)}>
                                    <i className="bi bi-send"></i>
                                </button>
                            </Tooltip>
                            <Tooltip tooltip="drop">
                                <button className="btn btn-sm btn-outline-danger"
                                        onClick={() => onDrop(message)}>
                                    <i className="bi bi-x-square"></i>
                                </button>
                            </Tooltip>
                        </div>}
                </td>
            </tr>)}
        </tbody>
    </table>;
}