import { Component } from "@angular/core";

@Component({
    selector: "app-icon-code-brackets",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
        }
        svg {
            width: 18px;
            height: 18px;
            display: block;
        }
    `,
    template: `
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <polyline points="7 8 3 12 7 16"></polyline>
            <polyline points="17 8 21 12 17 16"></polyline>
            <line x1="14" y1="4" x2="10" y2="20"></line>
        </svg>
    `,
})
export class IconCodeBracketsComponent {}
