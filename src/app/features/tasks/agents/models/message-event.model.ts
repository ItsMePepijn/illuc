import { Role } from "./role.model";

export interface MessageEvent {
    taskId: string;
    messageId: string;
    role: Role;
    content: string;
    isDelta: boolean;
    isFinal: boolean;
}
