import React, {Fragment, useState} from "react";
import {MessageInfo} from "../store/store";
import Tooltip from "./tooltip";
import {Json} from "./json";
import classNames from "classnames";

function messageFilter(message: MessageInfo, filterNodes: Set<string>, onlyUndelivered: boolean): boolean {
    if (onlyUndelivered && message.deliveredAt !== null) return false;
    return filterNodes.has(message.message.src) && filterNodes.has(message.message.dest);
}

export default function MessageView(props: {
    messages: MessageInfo[],
    filterNodes: Set<string>,
    highlighted: MessageInfo | null,
    setHighlighted: (messages: MessageInfo | null) => void,
    onDeliver: (messages: MessageInfo) => void,
    onDrop: (messages: MessageInfo) => void,
}) {
    const [showOnlyUndelivered, setShowOnlyUndelivered] = useState(true);
    return <div className="tool-pane">
        <div className="tool-pane-header">
            <div>
                <i className="bi bi-envelope"></i> Messages
            </div>
            <div className="form-check form-switch">
                <input className="form-check-input" type="checkbox" role="switch" id="showDeliveredMessages"
                       checked={!showOnlyUndelivered}
                       onChange={e => setShowOnlyUndelivered(!e.target.checked)}/>
                <label className="form-check-label" htmlFor="showDeliveredMessages">Show delivered
                    messages</label>
            </div>
        </div>
        <div className="tool-pane-content overflow-y-scroll">
            <MessageTable onlyUndelivered={showOnlyUndelivered} {...props}/>
        </div>
    </div>;
}

function MessageTable({
                          messages,
                          filterNodes,
                          onlyUndelivered,
                          ...props
                      }: {
    messages: MessageInfo[],
    filterNodes: Set<string>,
    onlyUndelivered: boolean,
    highlighted: MessageInfo | null,
    setHighlighted: (messages: MessageInfo | null) => void,
    onDeliver: (messages: MessageInfo) => void,
    onDrop: (messages: MessageInfo) => void,
}) {
    const shownMessages = messages.filter(m => messageFilter(m, filterNodes, onlyUndelivered));
    return <table className="table table-sm font-monospace table-hover align-middle">
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
                                                  showDeliveredAt={!onlyUndelivered}
                                                  {...props}/>)}
        </tbody>
    </table>;
}

function MessageRow({message, showDeliveredAt, highlighted, setHighlighted, onDeliver, onDrop}: {
    message: MessageInfo,
    showDeliveredAt: boolean,
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
            {showDeliveredAt && <td>{message.deliveredAt?.logical}</td>}
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