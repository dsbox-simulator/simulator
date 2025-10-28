import React, {useRef} from "react";
import {createPortal} from "react-dom";
import * as bootstrap from "bootstrap";
import classNames from "classnames";

interface ModalState {
    modal: bootstrap.Modal | null,
    closeConfirm: boolean,
}

export default function Modal({open, onClose, className, title, cancelButton, confirmButton, children}: {
    open: boolean,
    onClose: (confirmed: boolean) => void,
    className?: string,
    title: React.ReactNode,
    children: React.ReactNode
    cancelButton?: string,
    confirmButton?: string,
}) {
    const mdref = useRef<ModalState>({modal: null, closeConfirm: false});
    const enableModal = (element: HTMLElement | null) => {
        if (element === null) {
            if (mdref.current.modal !== null) mdref.current.modal.dispose();
            return;
        }
        mdref.current.modal = new bootstrap.Modal(element);
        mdref.current.modal.show();
        element.addEventListener("hidden.bs.modal", () => onClose(mdref.current.closeConfirm));
    };
    if (open) {
        return createPortal(<div ref={enableModal} className="modal fade">
            <div className={classNames("modal-dialog", className)}>
                <div className="modal-content">
                    <div className="modal-header">
                        <h1 className="modal-title fs-5">{title}</h1>
                        <button type="button" className="btn-close" data-bs-dismiss="modal" aria-label="Close"
                                onClick={_ => mdref.current.closeConfirm = false}></button>
                    </div>
                    <div className="modal-body">
                        {children}
                    </div>
                    <div className="modal-footer">
                        <button type="button" className="btn btn-danger" data-bs-dismiss="modal"
                                onClick={_ => mdref.current.closeConfirm = false}>{cancelButton || "Cancel"}
                        </button>
                        <button type="button" className="btn btn-success" data-bs-dismiss="modal"
                                onClick={_ => mdref.current.closeConfirm = true}>{confirmButton || "Save"}
                        </button>
                    </div>
                </div>
            </div>
        </div>, document.body);
    } else {
        return null;
    }
}