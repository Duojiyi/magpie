import { useEffect, useId, useRef } from "react";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  theme: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onClose: () => void;
}

const ConfirmDialog = ({
  open,
  title,
  message,
  theme,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onClose
}: ConfirmDialogProps) => {
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const messageId = useId();

  // a11y (P2): move focus into the dialog on open and let Escape dismiss it, so
  // destructive confirmations (e.g. clearing history) are safe/operable by keyboard
  // and screen-reader users. Hooks run unconditionally (before the early return).
  useEffect(() => {
    if (!open) return;
    confirmButtonRef.current?.focus();
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className={`confirm-dialog theme-${theme}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={messageId}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="confirm-dialog-title" id={titleId}>{title}</div>
        <div className="confirm-dialog-message" id={messageId}>{message}</div>
        <div className="confirm-dialog-buttons">
          <button className="confirm-dialog-button" onClick={onClose}>
            {cancelLabel}
          </button>
          <button
            ref={confirmButtonRef}
            className="confirm-dialog-button primary"
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ConfirmDialog;
