import { AgentKind } from "./agent-kind.model";

export interface AgentKindAvailability {
    kind: AgentKind;
    label: string;
    installed: boolean;
}
