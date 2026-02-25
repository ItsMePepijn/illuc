import { RequestQuestionOption } from "./request-question-option.model";

export interface RequestQuestion {
    id: string;
    header: string;
    question: string;
    isOther: boolean;
    isSecret: boolean;
    options: RequestQuestionOption[];
}
