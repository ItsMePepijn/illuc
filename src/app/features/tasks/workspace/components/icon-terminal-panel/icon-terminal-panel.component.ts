import { Component } from "@angular/core";

@Component({
    selector: "app-icon-terminal-panel",
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
            <rect x="3" y="5" width="18" height="14" rx="2"></rect>
            <polyline points="7 9 11 12 7 15"></polyline>
            <line x1="12.5" y1="15" x2="17" y2="15"></line>
        </svg>
    `,
})
export class IconTerminalPanelComponent {}
