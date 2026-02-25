import { Role } from "./role.model";
import { MessageStatus } from "./message-status.model";

export interface Message {
    id: string;
    role: Role;
    content: string;
    createdAt: string;
    status: MessageStatus;
}
