import { MessagePresentation } from "./message-presentation.model";
import { Role } from "./role.model";

export interface HistoryMessageEvent {
    messageId: string;
    role: Role;
    content: string;
    presentation: MessagePresentation;
    isDelta: boolean;
    isFinal: boolean;
}

export interface HistoryEvent {
    taskId: string;
    events: HistoryMessageEvent[];
}
