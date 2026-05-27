import type { CommandRequest, CommandResponse } from "../types";

export interface PendingCommand {
  request: CommandRequest;
  requestBytes: Uint8Array;
  resolve: (response: CommandResponse) => void;
  reject: (error: Error) => void;
  timer: number;
  acceptTimer?: number;
  accepted: boolean;
  attempts: number;
  events: CommandResponse[];
}

export function clearPendingTimers(pending: PendingCommand): void {
  window.clearTimeout(pending.timer);
  if (pending.acceptTimer !== undefined) {
    window.clearTimeout(pending.acceptTimer);
  }
}
