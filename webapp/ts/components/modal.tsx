import {createPortal} from "react-dom";
import classNames from "classnames";
import React from "react";

export default function Modal({onConfirm, id, className, title, cancelButton, confirmButton, children}: {
    onConfirm: () => void,
    id: string,
    className?: string,
    title: React.ReactNode,
    children: React.ReactNode
    cancelButton?: string,
    confirmButton?: string,
}) {
    return createPortal(<div id={id} className="modal fade">
        <div className={classNames("modal-dialog", className)}>
            <div className="modal-content">
                <div className="modal-header">
                    <h1 className="modal-title fs-5">{title}</h1>
                    <button type="button" className="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                </div>
                <div className="modal-body">
                    {children}
                </div>
                <div className="modal-footer">
                    <button type="button" className="btn btn-danger"
                            data-bs-dismiss="modal">{cancelButton || "Cancel"}</button>
                    <button type="button" className="btn btn-success" data-bs-dismiss="modal"
                            onClick={onConfirm}>{confirmButton || "Save"}
                    </button>
                </div>
            </div>
        </div>
    </div>, document.body);
}