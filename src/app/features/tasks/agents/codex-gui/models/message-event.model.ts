import { MessagePresentation } from "./message-presentation.model";
import { Role } from "./role.model";

export interface MessageEvent {
    taskId: string;
    messageId: string;
    role: Role;
    content: string;
    presentation: MessagePresentation;
    isDelta: boolean;
    isFinal: boolean;
}
