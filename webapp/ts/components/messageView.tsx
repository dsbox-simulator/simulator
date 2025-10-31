import React, {Fragment, useState} from "react";
import {LogInfo, MessageInfo} from "../store/store";
import Tooltip from "./tooltip";
import {Json} from "./json";
import classNames from "classnames";

function messageFilter(message: MessageInfo, onlyUndelivered: boolean): boolean {
    if (onlyUndelivered && message.deliveredAt !== null) return false;
    return !(message.message.src === "core" || message.message.dest === "core");
}

export default function MessageView({messages, onlyUndelivered, highlighted, setHighlighted, onDeliver, onDrop}: {
    messages: MessageInfo[],
    onlyUndelivered: boolean,
    highlighted: MessageInfo | null,
    setHighlighted: (messages: MessageInfo | null) => void,
    onDeliver: (messages: MessageInfo) => void,
    onDrop: (messages: MessageInfo) => void,
}) {
    const shownMessages = messages.filter(m => messageFilter(m, onlyUndelivered));
    return <table className="table table-small font-monospace table-hover">
        <thead>
        <tr className="sticky-top">
            <th></th>
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
        {shownMessages.map(message => <MessageRow key={message.sentAt.logical}
                                                  message={message}
                                                  onlyUndelivered={onlyUndelivered}
                                                  highlighted={highlighted}
                                                  setHighlighted={setHighlighted}
                                                  onDeliver={onDeliver}
                                                  onDrop={onDrop}/>)}
        </tbody>
    </table>;
}

function MessageRow({message, onlyUndelivered, highlighted, setHighlighted, onDeliver, onDrop}: {
    message: MessageInfo,
    onlyUndelivered: boolean,
    highlighted: MessageInfo | null,
    setHighlighted: (messages: MessageInfo | null) => void,
    onDeliver: (messages: MessageInfo) => void,
    onDrop: (messages: MessageInfo) => void,
}) {
    const [open, setOpen] = useState(false);
    return <>
        <tr className={classNames({
            "d-none": message.dropped,
            "table-secondary": message.sentAt === highlighted?.sentAt
        })}
            onMouseEnter={() => setHighlighted(message)}
            onMouseLeave={() => setHighlighted(null)}>
            <td><a role="button" onClick={() => setOpen(!open)}><i
                className={classNames("bi", {"bi-plus-square": !open, "bi-dash-square": open})}></i></a></td>
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
        </tr>
        {open && <tr>
            <td colSpan={999}><Json json={message.message}/></td>
        </tr>}
    </>;
}