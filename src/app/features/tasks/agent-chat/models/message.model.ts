import { MessagePresentation } from "./message-presentation.model";
import { Role } from "./role.model";
import { MessageStatus } from "./message-status.model";

export interface Message {
    id: string;
    role: Role;
    content: string;
    presentation: MessagePresentation;
    createdAt: string;
    status: MessageStatus;
}
